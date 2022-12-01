// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *   Zongwu Li <lzw32321226@163.com>
 *
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/namei.h>
#include <linux/uprobes.h>
#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/proc_fs.h>
#include <linux/elf.h>
#include <linux/dcache.h>
#include <linux/file.h>
#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/string.h>
#include <linux/spinlock.h>
#include <linux/slab.h>
#include <linux/version.h>

#include <asm/ptrace.h>

#include "kmod.h"
#include "compiler.h"
#include "common.h"
#include "patch-uprobe.h"
#include "upatch-ioctl.h"

#include "asm/hijack.h"

struct elf_path {
    char name[PATH_MAX];
    unsigned int count;
    loff_t entry_offset;
    struct list_head list;
};

static DEFINE_MUTEX(compiler_steps_lock);
static LIST_HEAD(compiler_steps_list);

#define FILENAME_ID_LEN 128
DEFINE_SPINLOCK(filename_lock);
static unsigned long filename_id = 0;

static LIST_HEAD(compiler_paths_list);
static DEFINE_MUTEX(compiler_paths_lock);

static LIST_HEAD(assembler_paths_list);
static DEFINE_MUTEX(assembler_paths_lock);

static struct list_head *get_elf_list(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &compiler_paths_list;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &assembler_paths_list;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static struct mutex *get_elf_lock(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &compiler_paths_lock;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &assembler_paths_lock;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static struct elf_path *__get_elf_path(unsigned int cmd, char *name)
{
    struct elf_path *ep;
    struct list_head *elf_list;

    elf_list = get_elf_list(cmd);
    if (elf_list) {
        list_for_each_entry(ep, elf_list, list) {
            if (strncmp(ep->name, name, PATH_MAX) == 0) {
                return ep;
            }
        }
    }
    return NULL;
}

struct elf_path *get_elf_path(unsigned int cmd, char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        mutex_unlock(lock);
    }
    return ep;
}

static void inline __delete_elf_path(unsigned int cmd, char *name)
{
    struct elf_path *ep;

    if (!name)
        return;

    ep = __get_elf_path(cmd, name);
    if (ep) {
        list_del(&ep->list);
        kfree(ep);
    }
}

void delete_elf_path(unsigned int cmd, char *name)
{
    struct mutex *lock;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        __delete_elf_path(cmd, name);
        mutex_unlock(lock);
    }
}

void clear_compiler_path(void)
{
    struct elf_path *ep, *tmp;
    list_for_each_entry_safe(ep, tmp, &compiler_paths_list, list)
        delete_elf_path(UPATCH_UNREGISTER_COMPILER, ep->name);
}

void clear_assembler_path(void)
{
    struct elf_path *ep, *tmp;
    list_for_each_entry_safe(ep, tmp, &assembler_paths_list, list)
        delete_elf_path(UPATCH_UNREGISTER_ASSEMBLER, ep->name);
}

static int generate_file_name(char *buf, int buf_len)
{
    unsigned long id;
    size_t len;

    if (!buf || buf_len < FILENAME_ID_LEN)
        return -EINVAL;

    spin_lock(&filename_lock);
    filename_id ++;
    id = filename_id;
    spin_unlock(&filename_lock);

    snprintf(buf, buf_len, "%ld", id);

    len = strlen(buf);
    buf[len] = '.';
    buf[len + 1] = 'o';
    buf[len + 2] = '\0';

    return 0;
}

static struct compiler_step *__get_compiler_setp(char *name)
{
    struct compiler_step *cs;
    list_for_each_entry(cs, &compiler_steps_list, list)
        if (strncmp(cs->name, name, COMPILER_STEP_MAX_LEN) == 0)
            return cs;
    return NULL;
}

static struct compiler_step *get_compiler_setp(char *name)
{
    struct compiler_step *cs;
    mutex_lock(&compiler_steps_lock);
    cs = __get_compiler_setp(name);
    mutex_unlock(&compiler_steps_lock);
    return cs;
}

static int __register_compiler_step(char *name, cs_handler_t step_handler)
{
    struct compiler_step *cs;

    if (!name || !step_handler)
        return -EINVAL;

    if (__get_compiler_setp(name))
        return 0;

    cs = kzalloc(sizeof(*cs), GFP_KERNEL);
    if (!cs)
        return -ENOMEM;

    strncpy(cs->name, name, COMPILER_STEP_MAX_LEN);
    cs->step_handler = step_handler;
    list_add(&cs->list, &compiler_steps_list);
    return 0;
}

int register_compiler_step(char *name, cs_handler_t step_handler)
{
    int ret;
    mutex_lock(&compiler_steps_lock);
    ret = __register_compiler_step(name, step_handler);
    mutex_unlock(&compiler_steps_lock);
    return ret;
}

static void __unregister_compiler_setp(char *name)
{
    struct compiler_step *cs;

    if (!name)
        return;

    cs = __get_compiler_setp(name);
    if (cs) {
        list_del(&cs->list);
        kfree(cs);
    }
}

void unregister_compiler_setp(char *name)
{
    mutex_lock(&compiler_steps_lock);
    __unregister_compiler_setp(name);
    mutex_unlock(&compiler_steps_lock);
}

void clear_compiler_step(void)
{
    struct compiler_step *cs, *tmp;
    mutex_lock(&compiler_steps_lock);
    list_for_each_entry_safe(cs, tmp, &compiler_steps_list, list)
        __unregister_compiler_setp(cs->name);
    mutex_unlock(&compiler_steps_lock);
}

static int obtain_parameter_pointer(char __user **pointer_array, char *name,
    unsigned long *addr_pointer, unsigned long *next_pointer)
{
    char *__buffer;
    unsigned long pointer_addr;
    size_t len = strlen(name);

    if (!pointer_array)
        return -EINVAL;

    __buffer = kmalloc(len + 1, GFP_KERNEL);
    if (!__buffer)
        return -ENOMEM;

    __buffer[len] = '\0';

    if (addr_pointer)
        *addr_pointer = 0;

    if (next_pointer)
        *next_pointer = 0;

    for (;;) {
        /* get pointer address first */
        if (get_user(pointer_addr, (unsigned long __user *)pointer_array))
            break;
        pointer_array ++;

        if (!(const char __user *)pointer_addr)
            break;

        if (copy_from_user(__buffer, (void __user *)pointer_addr, len))
            break;

        /* if not matched, continue */
        if (strncmp(__buffer, name, len))
            continue;

        pointer_array --;
        if (addr_pointer)
            *addr_pointer = (unsigned long)(unsigned long __user *)pointer_array;

        pointer_array ++;
        if (next_pointer)
            *next_pointer = (unsigned long)(unsigned long __user *)pointer_array;

        break;
    }

    if (__buffer)
        kfree(__buffer);

    return 0;
}

static int obtain_parameter_addr(char __user **pointer_array, char *name,
    unsigned long *addr, unsigned long *next_addr)
{
    int ret;
    unsigned long tmp;
    unsigned long addr_pointer, next_pointer;

    if (addr)
        *addr = 0;

    if (next_addr)
        *next_addr = 0;

    ret = obtain_parameter_pointer(pointer_array, name, &addr_pointer, &next_pointer);
    if (ret)
        return ret;

    if (addr && addr_pointer != 0
        && !get_user(tmp, (unsigned long __user *)addr_pointer))
        *addr = tmp;

    if (next_addr && next_pointer != 0
        && !get_user(tmp, (unsigned long __user *)next_pointer))
        *next_addr = tmp;

    return 0;
}

static struct compiler_step *check_env_for_step(char __user **envp,
    unsigned long *cmd_addr)
{
    int ret;
    size_t len;
    char __env[COMPILER_CMD_MAX_LEN];
    unsigned long env_pos;

    ret = obtain_parameter_addr(envp, COMPILER_CMD_ENV, cmd_addr, NULL);
    if (ret || *cmd_addr == 0) {
        pr_debug("no valid env found for %s \n", COMPILER_CMD_ENV);
        return NULL;
    }

    env_pos = *cmd_addr;
    len = strnlen_user((void __user *)env_pos, MAX_ARG_STRLEN);

    if (len >= COMPILER_CMD_MAX_LEN)
        return NULL;

    if (copy_from_user(__env, (void __user *)env_pos, len))
        return NULL;

    return get_compiler_setp(&__env[COMPILER_CMD_ENV_LEN + 1]);
}

static int unlink_filename(const char *filename)
{
    struct path path;
    struct inode *parent_inode;
    int ret;

	ret = kern_path(filename, 0, &path);
	if (ret)
		return ret;

    parent_inode = path.dentry->d_parent->d_inode;
    inode_lock(parent_inode);
    #if LINUX_VERSION_CODE > KERNEL_VERSION(5, 10, 0)
        ret = vfs_unlink(mnt_user_ns(path.mnt), parent_inode, path.dentry, NULL);
    #else
        ret = vfs_unlink(parent_inode, path.dentry, NULL);
    #endif
    inode_unlock(parent_inode);

    return ret;
}

/* check init_symlink for more info */
static int create_symlink(const char *oldname, const char *newname)
{
	struct dentry *dentry;
	struct path path;
	int error;

    /* we do not care about its return value */
    unlink_filename(newname);

    dentry = kern_path_create(AT_FDCWD, newname, &path, 0);
	if (IS_ERR(dentry))
        return PTR_ERR(dentry);

    #if LINUX_VERSION_CODE > KERNEL_VERSION(5, 10, 0)
	    error = vfs_symlink(mnt_user_ns(path.mnt), path.dentry->d_inode, dentry, oldname);
    #else
	    error = vfs_symlink(path.dentry->d_inode, dentry, oldname);
    #endif
	done_path_create(&path, dentry);
	return error;
}

/*
 * To generate the new object filepath, three ways:
 * 1. append source path from '-o' to the output dir
 * 2. generate new filename based on the HASH mechansim
 * 3. generate new filename using a id number (we use this approach)
 */
static int rewrite_object_path(char __user **argv, char __user **envp)
{
    int ret;
    size_t len;
    char *object_path = NULL;
    char *out_dir = NULL;
    char *kernel_new_path = NULL;
    char *new_path = NULL;
    char filename_buff[FILENAME_ID_LEN];
    unsigned long arg_pointer;
    unsigned long arg_addr, dir_addr;

    ret = obtain_parameter_pointer(argv, "-o", NULL, &arg_pointer);
    if (ret || arg_pointer == 0) {
        pr_debug("no valid object object_path found - %d \n", ret);
        ret = 0;
        goto out;
    }

    ret = -EFAULT;
    if (get_user(arg_addr, (unsigned long __user *)arg_pointer))
        goto out;

    ret = -ENOMEM;
    object_path = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!object_path)
        goto out;

    ret = copy_para_from_user(arg_addr, object_path, PATH_MAX);
    if (ret)
        goto out;

    ret = obtain_parameter_addr(envp, ASSEMBLER_DIR_ENV, &dir_addr, NULL);
    if (ret || dir_addr == 0) {
        pr_warn("no valid %s found %s \n", ASSEMBLER_DIR_ENV, object_path);
        ret = -EINVAL;
        goto out;
    }

    ret = -ENOMEM;
    out_dir = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!out_dir)
        goto out;

    ret = copy_para_from_user((unsigned long)((char *)dir_addr + ASSEMBLER_DIR_ENV_LEN + 1),
        out_dir, PATH_MAX);
    if (ret)
        goto out;

    ret = generate_file_name(filename_buff, FILENAME_ID_LEN);
    if (ret)
        goto out;

    len = strlen(out_dir) + 1 + strlen(filename_buff) + 1;

    ret = -ENOMEM;
    kernel_new_path = kmalloc(len, GFP_KERNEL);
    if (!kernel_new_path)
        goto out;

    new_path = (void *)vm_mmap(NULL, 0, len,
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)new_path))
        goto out;

    ret = -ENOMEM;
    strncpy(kernel_new_path, out_dir, strlen(out_dir));
    strncpy(kernel_new_path + strlen(out_dir), "/", 1);
    strncpy(kernel_new_path + strlen(out_dir) + 1, filename_buff, strlen(filename_buff));
    strncpy(kernel_new_path + strlen(out_dir) + 1 + strlen(filename_buff), "\0", 1);

    if (copy_to_user(new_path, kernel_new_path, len))
        goto out;

    /* modify path of output name */
    if (put_user((unsigned long)new_path, (unsigned long *)arg_pointer))
        goto out;

    pr_debug("exist file name is %s \n", kernel_new_path);
    pr_debug("link file name is %s \n", object_path);

    ret = create_symlink(kernel_new_path, object_path);
    if (ret) {
        pr_err("create symbol link for linker failed - %d \n", ret);
        goto out;
    }

    ret = 0;
    goto out_normal;

out:
    if (new_path && !IS_ERR((void *)new_path))
        vm_munmap((unsigned long)new_path, len);
out_normal:
    if (object_path)
        kfree(object_path);
    if (out_dir)
        kfree(out_dir);
    if (kernel_new_path)
        kfree(kernel_new_path);
    return ret;
}

static inline char __user **get_argv_from_regs(struct pt_regs *regs)
{
    unsigned long stack_pointer = user_stack_pointer(regs);
    return (void *)(stack_pointer + sizeof(unsigned long));
}

static inline char __user **get_env_from_regs(struct pt_regs *regs)
{
    int argc;
    unsigned long stack_pointer = user_stack_pointer(regs);
    char __user **argv = get_argv_from_regs(regs);

    if (get_user(argc, (int *)stack_pointer)) {
        pr_err("unable to read argc from stack pointer \n");
        return NULL;
    }

    return (void *)((unsigned long)argv + (argc + 1) * sizeof(unsigned long));
}

/* check https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf for initial stack */
static int uprobe_compiler_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long cmd_addr;
    struct compiler_step *cs;
    char __user **envp = get_env_from_regs(regs);

    if (!envp)
        return 0;

    cs = check_env_for_step(envp, &cmd_addr);
    if (!cs) {
        pr_debug("no upatch cmd found \n");
        return 0;
    }

    return cs->step_handler(cs, regs, (char __user *)cmd_addr);
}

static struct uprobe_consumer uprobe_compiler_consumer = {
    .handler = uprobe_compiler_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

static int uprobe_assembler_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    int ret;
    char __user **argv = get_argv_from_regs(regs);
    char __user **envp = get_env_from_regs(regs);

    if (!argv || !envp)
        return 0;

    ret = rewrite_object_path(argv, envp);
    if (ret) {
        pr_warn("rewrite object path failed - %d \n", ret);
        run_exit_syscall(regs, ret);
    }

    return 0;
}

static struct uprobe_consumer uprobe_assembler_consumer = {
    .handler = uprobe_assembler_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

static int __init compiler_step_init(void)
{
    int ret;

    ret = register_compiler_step(CMD_SOURCE_ENTER, compiler_args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_SOURCE_AFTER, compiler_args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_PATCHED_ENTER, compiler_args_handler);
    if (ret)
        goto out;

    ret = register_compiler_step(CMD_PATCHED_AFTER, compiler_args_handler);
    if (ret)
        goto out;

out:
    return ret;
}

static int __unregister_uprobe(unsigned int cmd, struct elf_path *ep, struct uprobe_consumer *uc)
{
    int ret;
    struct inode *inode;
    struct path path;

    /* if path is null, return directly */
    if (strlen(ep->name) == 0)
        return 0;

    ret = kern_path(ep->name, LOOKUP_FOLLOW, &path);
    if (ret) {
        pr_err("kernel path failed - %d \n", ret);
        goto out;
    }
    inode = path.dentry->d_inode;

    pr_debug("unregister uprobe for %s \n", ep->name);
    uprobe_unregister(inode, ep->entry_offset, uc);
    if (cmd == UPATCH_UNREGISTER_COMPILER || cmd == UPATCH_UNREGISTER_ASSEMBLER) {
        delete_elf_path(cmd, ep->name);
    }
out:
    return ret;
}

static int __elf_check(struct file *file, loff_t *entry_offset)
{
    Elf_Ehdr elf_header;
    int ret;
    elf_addr_t min_addr;

    ret = kernel_read(file, &elf_header, sizeof(elf_header), 0);
    if (ret != sizeof(elf_header)) {
        pr_err("kernel read failed - %d \n", ret);
        ret = -ENOMEM;
        goto out;
    }

    min_addr = calculate_load_address(file, false);
    if (min_addr == -1) {
        pr_err("no valid segment found \n");
        ret = -EINVAL;
        goto out;
    }

    *entry_offset = elf_header.e_entry - min_addr;

    ret = 0;
out:
    return ret;
}

static int elf_check(const char *buf, char *elf_path, loff_t *entry_offset)
{
    struct file *file;
    int ret;
    char *p;

    file = filp_open(buf, O_RDONLY, 0);
    if (IS_ERR(file)) {
        ret = PTR_ERR(file);
        pr_err("open elf failed - %d \n", ret);
        goto out;
    }

    p = file_path(file, elf_path, PATH_MAX);
    if (IS_ERR(p)) {
        ret = PTR_ERR(p);
        pr_err("obtain path failed - %d \n", ret);
        goto put_file;
    }

    memmove(elf_path, p, strlen(p) + 1);
    pr_debug("elf path is %s len is %lu \n", elf_path, strlen(elf_path));

    ret = __elf_check(file, entry_offset);
put_file:
    fput(file);
out:
    return ret;
}

static int __register_uprobe(const char *buf, unsigned int cmd, struct elf_path *ep, struct uprobe_consumer *uc)
{
    int ret;
    struct path path;
    struct inode *inode;

    ret = elf_check(buf, ep->name, &ep->entry_offset);
    if (ret)
        goto out;

    ret = kern_path(ep->name, LOOKUP_FOLLOW, &path);
    if (ret) {
        pr_err("kernel path failed - %d \n", ret);
        goto out;
    }
    inode = path.dentry->d_inode;

    pr_debug("register uprobe for %s \n", buf);
    ret = uprobe_register(inode, ep->entry_offset, uc);
    if (ret) {
        pr_err("uprobe register failed - %d \n", ret);
        if (cmd == UPATCH_REGISTER_COMPILER)
            delete_elf_path(UPATCH_UNREGISTER_COMPILER, ep->name);
        else
            delete_elf_path(UPATCH_UNREGISTER_ASSEMBLER, ep->name);
        goto out;
    }

    ret = 0;
out:
    return ret;
}

int handle_compiler_cmd(unsigned long user_addr, unsigned int cmd)
{
    int ret;
    char *path = NULL;
    struct elf_path *ep = NULL;

    path = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!path)
        return -ENOMEM;

    ret = copy_para_from_user(user_addr, path, PATH_MAX);
    if (ret)
        goto out;

    ep = get_elf_path(cmd, path);

    switch (cmd)
    {
    case UPATCH_REGISTER_COMPILER:
        if (!ep) {
            ep = kzalloc(sizeof(*ep), GFP_KERNEL);
            if (!ep)
                return -ENOMEM;

            strncpy(ep->name, path, PATH_MAX);
            ep->count = 1;
            ep->entry_offset = 0;
            list_add(&ep->list, &compiler_paths_list);
            ret = __register_uprobe(path, cmd, ep, &uprobe_compiler_consumer);
        } else {
            ep->count++;
        }
        break;

    case UPATCH_UNREGISTER_COMPILER:
        if (ep) {
            ep->count--;
            if (!ep->count)
                ret = __unregister_uprobe(cmd, ep, &uprobe_compiler_consumer);
        }
        break;

    case UPATCH_REGISTER_ASSEMBLER:
        if (!ep) {
            ep = kzalloc(sizeof(*ep), GFP_KERNEL);
            if (!ep)
                return -ENOMEM;

            strncpy(ep->name, path, PATH_MAX);
            ep->count = 1;
            ep->entry_offset = 0;
            list_add(&ep->list, &assembler_paths_list);
            ret = __register_uprobe(path, cmd, ep, &uprobe_assembler_consumer);
        } else {
            ep->count++;
        }
        break;

    case UPATCH_UNREGISTER_ASSEMBLER:
        if (ep) {
            ep->count--;
            if (!ep->count)
                ret = __unregister_uprobe(cmd, ep, &uprobe_assembler_consumer);
        }
        break;

    default:
        ret = -ENOTTY;
        break;
    }

out:
    if (path)
        kfree(path);
    return ret;
}

int __init compiler_hack_init(void)
{
    int ret;

    ret = compiler_step_init();
    if (ret) {
        pr_err("compiler step register failed - %d \n", ret);
        goto out;
    }

out:
    return ret;
}

void __exit compiler_hack_exit(void)
{
    clear_compiler_path();
    clear_assembler_path();
    clear_compiler_step();
}
