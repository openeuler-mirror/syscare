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
#include <linux/vmalloc.h>
#include <linux/ctype.h>

#include <asm/ptrace.h>

#include "kmod.h"
#include "compiler.h"
#include "common.h"
#include "patch-uprobe.h"
#include "patch-syscall.h"
#include "upatch-ioctl.h"

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

static LIST_HEAD(link_paths_list);
static DEFINE_MUTEX(link_paths_lock);

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
        pr_debug("no valid %s found %s \n", ASSEMBLER_DIR_ENV, object_path);
        ret = 0;
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

static const char *get_real_path(const char *path_buff, unsigned int buff_size)
{
    int err;
    struct path path;
    const char* real_name = NULL;

    err = kern_path(path_buff, LOOKUP_FOLLOW, &path);
    if (!err)
        real_name = d_path(&path, (void *)path_buff, buff_size);
    return real_name;
}

static bool is_upatch_object_name(const char *name)
{
    unsigned int len = strlen(name);

    if (len >= 3 && name[len - 1] == 'o'
        && name[len - 2] == '.' && isdigit(name[len - 3]))
        return true;
    return false;
}

static void write_one_object_name(struct file *log_file,
    const char *binary_name, const char *object_buff)
{
    ssize_t	nwritten;
    unsigned int len;
    char *content = NULL;
    const char *object_name = NULL;

    object_name = get_real_path(object_buff, PATH_MAX);
    if (object_name == NULL || IS_ERR(object_name))
        return;

    if (!is_upatch_object_name(object_name))
        return;

    len = strlen(binary_name) + 4 + strlen(object_name) + 2;
    content = vmalloc(len);
    if (!content)
        goto out;

    memcpy(content, binary_name, strlen(binary_name));
    memcpy(content + strlen(binary_name), "::::", 4);
    memcpy(content + strlen(binary_name) + 4, object_name, strlen(object_name));
    memcpy(content + strlen(binary_name) + 4 + strlen(object_name), "\n", 1);
    content[len - 1] = 0;

    nwritten = kernel_write(log_file, content, strlen(content), NULL);
    if (nwritten != strlen(content))
        pr_err("write link log failed - %ld \n", nwritten);

out:
    if (content)
        vfree(content);
    return;
}

static void handle_tmpname_file(struct file *log_file,
    const char *binary_name, const char *file_path)
{
    int ret;
    unsigned int len, tmp;
    struct file* filp = NULL;
    char *content = NULL;
    char *object_buf = NULL;

    filp = filp_open(file_path, O_RDONLY, 0);
    if (filp == NULL || IS_ERR(filp))
        goto out;

    len = filp->f_inode->i_size;
    content = vmalloc(len);
    if (!content)
        goto out;

    object_buf = vmalloc(PATH_MAX);
    if (!object_buf)
        goto out;

    ret = kernel_read(filp, content, len, 0);
    if (ret != len) {
        pr_err("read tmp file failed for linker \n");
        goto out;
    }

    for (tmp = 0; tmp < len; tmp ++) {
        if (content[tmp] == '\r' || content[tmp] == '\n')
            content[tmp] = '\0';
    }

    tmp = 0;
    while (tmp < len) {
        unsigned int obj_len = strlen(content + tmp);
        if (obj_len < PATH_MAX) {
            memcpy(object_buf, content + tmp, obj_len);
            object_buf[obj_len] = '\0';
            write_one_object_name(log_file, binary_name, object_buf);
        }
        tmp += obj_len + 1;
    }

out:
    if (object_buf)
        vfree(object_buf);
    if (content)
        vfree(content);
    if (filp && !IS_ERR(filp))
        filp_close(filp, NULL);
    return;
}

static int handle_object_name(struct file *log_file, const char *binary_name, char __user **pointer_array)
{
    unsigned long pointer_addr;
    char *__buffer = NULL;
    unsigned int len;

    if (!pointer_array)
        return -EINVAL;

    __buffer = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!__buffer)
        return -ENOMEM;

    for (;;) {
        if (get_user(pointer_addr, (unsigned long __user *)pointer_array))
            break;

        if (!(const char __user *)pointer_addr)
            break;

        len = strnlen_user((void __user *)pointer_addr, MAX_ARG_STRLEN);
        if (copy_from_user(__buffer, (void __user *)pointer_addr, len))
            break;

        pointer_array ++;

        if (__buffer[0] == '@') {
            handle_tmpname_file(log_file, binary_name, &__buffer[1]);
            continue;
        }

        write_one_object_name(log_file, binary_name, __buffer);
    }

    if (__buffer)
        kfree(__buffer);
    return 0;
}

static int obtain_link_info(char __user **argv, char __user **envp)
{
    int ret;
    unsigned long arg_addr, log_addr;
    char *path_buf = NULL;
    const char *name = NULL;
    struct file *log_file = NULL, *tmp_file = NULL;

    ret = -ENOMEM;
    path_buf = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!path_buf)
        goto out;

    ret = obtain_parameter_addr(envp, LINK_PATH_ENV, &log_addr, NULL);
    if (ret || log_addr == 0)
        goto out;

    ret = copy_para_from_user((unsigned long)((char *)log_addr + LINK_PATH_ENV_LEN + 1),
        path_buf, PATH_MAX);
    if (ret)
        goto out;

    log_file = filp_open(path_buf, O_WRONLY | O_SYNC | O_CREAT | O_APPEND, 0600);
    if (IS_ERR(log_file)) {
        ret = PTR_ERR(log_file);
        pr_err("open log file %s failed - %d \n", path_buf, ret);
        goto out;
    }

    ret = obtain_parameter_addr(argv, "-o", NULL, &arg_addr);
    /* ATTENTION: if it fails to find the argument, it will work as a.out. */
    if (ret || arg_addr == 0) {
        memcpy(path_buf, "a.out", 6);
        goto out_name;
    }

    ret = copy_para_from_user(arg_addr, path_buf, PATH_MAX);
    if (ret)
        goto out;

out_name:
    tmp_file = filp_open(path_buf, O_CREAT | O_RDONLY, 0600);
    if (!IS_ERR(tmp_file))
        filp_close(tmp_file, NULL);

    ret = -ENOENT;
    name = get_real_path(path_buf, PATH_MAX);
    if (name == NULL || IS_ERR(name)) {
        pr_err("get binary name failed \n");
        goto out;
    }

    ret = handle_object_name(log_file, name, argv);
    if (ret)
        goto out;

    ret = 0;
    goto out;

out:
    if (log_file && !IS_ERR(log_file))
        filp_close(log_file, NULL);
    if (path_buf)
        kfree(path_buf);
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
        exit_syscall(regs, ret);
    }

    return 0;
}

static struct uprobe_consumer uprobe_assembler_consumer = {
    .handler = uprobe_assembler_handler,
    .ret_handler = NULL,
    .filter = uprobe_default_filter,
};

static int uprobe_link_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    int ret;
    char __user **argv = get_argv_from_regs(regs);
    char __user **envp = get_env_from_regs(regs);

    if (!argv || !envp)
        return 0;

    ret = obtain_link_info(argv, envp);
    if (ret) {
        pr_warn("obtain link info failed - %d \n", ret);
        exit_syscall(regs, ret);
    }

    return 0;
}

static struct uprobe_consumer uprobe_link_consumer = {
    .handler = uprobe_link_handler,
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

static int elf_check(char *elf_path, loff_t *entry_offset)
{
    struct file *file;
    int ret;
    char *p;

    file = filp_open(elf_path, O_RDONLY, 0);
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
    filp_close(file, NULL);
out:
    return ret;
}

static struct list_head *get_elf_list(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &compiler_paths_list;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &assembler_paths_list;
    } else if ((cmd == UPATCH_REGISTER_LINK) || (cmd == UPATCH_UNREGISTER_LINK)) {
        return &link_paths_list;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static struct uprobe_consumer *get_uprobe_consumer(unsigned int cmd)
{
    if ((cmd == UPATCH_REGISTER_COMPILER) || (cmd == UPATCH_UNREGISTER_COMPILER)) {
        return &uprobe_compiler_consumer;
    } else if ((cmd == UPATCH_REGISTER_ASSEMBLER) || (cmd == UPATCH_UNREGISTER_ASSEMBLER)) {
        return &uprobe_assembler_consumer;
    } else if ((cmd == UPATCH_REGISTER_LINK) || (cmd == UPATCH_UNREGISTER_LINK)) {
        return &uprobe_link_consumer;
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
    } else if ((cmd == UPATCH_REGISTER_LINK) || (cmd == UPATCH_UNREGISTER_LINK)) {
        return &link_paths_lock;
    } else {
        pr_warn("invalid command for upatch cmd. \n");
        return NULL;
    }
}

static int __register_uprobe(unsigned int cmd, struct elf_path *ep, struct uprobe_consumer *uc)
{
    int ret;
    struct path path;
    struct inode *inode;

    ret = elf_check(ep->name, &ep->entry_offset);
    if (ret)
        goto out;

    ret = kern_path(ep->name, LOOKUP_FOLLOW, &path);
    if (ret) {
        pr_err("kernel path failed - %d \n", ret);
        goto out;
    }
    inode = path.dentry->d_inode;

    pr_debug("register uprobe for %s \n", ep->name);
    ret = uprobe_register(inode, ep->entry_offset, uc);
out:
    return ret;
}

static struct elf_path *__get_elf_path(unsigned int cmd, const char *name)
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

static int inline __delete_elf_path(unsigned int cmd, const char *name)
{
    struct elf_path *ep;
    int ret;
    struct uprobe_consumer  *uc = get_uprobe_consumer(cmd);

    if (!uc)
        return -ENOTTY;

    ep = __get_elf_path(cmd, name);
    if (ep) {
        ret = __unregister_uprobe(cmd, ep, uc);
        if (ret)
            return ret;
        list_del(&ep->list);
        kfree(ep);
    }
    return 0;
}

static int inline __add_elf_path(unsigned int cmd, const char *name)
{
    struct elf_path *ep;
    int ret;
    struct list_head *elf_list = get_elf_list(cmd);
    struct uprobe_consumer  *uc = get_uprobe_consumer(cmd);

    if (!uc)
        return -ENOTTY;

    if (!elf_list)
        return -ENOTTY;

    ep = kzalloc(sizeof(*ep), GFP_KERNEL);
    if (!ep)
        return -ENOTTY;

    strncpy(ep->name, name, PATH_MAX);
    ep->count = 1;
    ep->entry_offset = 0;

    ret = __register_uprobe(cmd, ep, uc);
    if (ret)
        return ret;

    list_add(&ep->list, elf_list);

    return 0;
}

struct elf_path *get_elf_path(unsigned int cmd, const char *name)
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

int delete_elf_path(unsigned int cmd, const char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;
    int ret = 0;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        if (ep) {
            ep->count--;
            if (!ep->count)
                ret = __delete_elf_path(cmd, name);
        }
        mutex_unlock(lock);
    }
    return ret;
}

int add_elf_path(unsigned int cmd, const char *name)
{
    struct mutex *lock;
    struct elf_path *ep = NULL;
    int ret = 0;

    lock = get_elf_lock(cmd);
    if (lock) {
        mutex_lock(lock);
        ep = __get_elf_path(cmd, name);
        if (!ep) {
            ret = __add_elf_path(cmd, name);
        } else {
            ep->count++;
        }
        mutex_unlock(lock);
    }
    return ret;
}

int handle_compiler_cmd(unsigned long user_addr, unsigned int cmd)
{
    int ret;
    char *path_buf = NULL;
    const char* real_path;

    path_buf = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!path_buf)
        return -ENOMEM;

    ret = copy_para_from_user(user_addr, path_buf, PATH_MAX);
    if (ret)
        goto out;

    real_path = get_real_path(path_buf, PATH_MAX);
    if (real_path == NULL || IS_ERR(real_path)) {
        pr_err("get real_path failed: %u \n", cmd);
        goto out;
    }

    switch (cmd)
    {
    case UPATCH_REGISTER_COMPILER:
    case UPATCH_REGISTER_ASSEMBLER:
    case UPATCH_REGISTER_LINK:
        ret = add_elf_path(cmd, real_path);
        break;

    case UPATCH_UNREGISTER_COMPILER:
    case UPATCH_UNREGISTER_ASSEMBLER:
    case UPATCH_UNREGISTER_LINK:
        ret = delete_elf_path(cmd, real_path);
        break;

    default:
        ret = -ENOTTY;
        break;
    }

out:
    if (path_buf)
        kfree(path_buf);
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

void clear_compiler_path(void)
{
    struct elf_path *ep, *tmp;

    mutex_lock(&compiler_paths_lock);
    list_for_each_entry_safe(ep, tmp, &compiler_paths_list, list) {
        __delete_elf_path(UPATCH_UNREGISTER_COMPILER, ep->name);
    }
    mutex_unlock(&compiler_paths_lock);
}

void clear_assembler_path(void)
{
    struct elf_path *ep, *tmp;
    mutex_lock(&assembler_paths_lock);
    list_for_each_entry_safe(ep, tmp, &assembler_paths_list, list) {
        __delete_elf_path(UPATCH_UNREGISTER_ASSEMBLER, ep->name);
    }
    mutex_unlock(&assembler_paths_lock);
}

void clear_link_path(void)
{
    struct elf_path *ep, *tmp;
    mutex_lock(&link_paths_lock);
    list_for_each_entry_safe(ep, tmp, &link_paths_list, list) {
        __delete_elf_path(UPATCH_UNREGISTER_LINK, ep->name);
    }
    mutex_unlock(&link_paths_lock);
}

void __exit compiler_hack_exit(void)
{
    clear_compiler_path();
    clear_assembler_path();
    clear_link_path();
    clear_compiler_step();
}
