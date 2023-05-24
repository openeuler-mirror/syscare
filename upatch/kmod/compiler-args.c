// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/slab.h>
#include <linux/namei.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/uprobes.h>
#include <linux/file.h>
#include <linux/version.h>

#include <asm/uaccess.h>
#include <asm/ptrace.h>

#include "compiler-args.h"
#include "patch-syscall.h"
#include "common.h"

static const char *compiler_append_args[] = {
    "-gdwarf", /* obatain debug information */
    "-ffunction-sections",
    "-fdata-sections",
    "NULL",
};

#define COMPILER_APPEND_APGS_SIZE (sizeof(compiler_append_args) / sizeof(char *) - 1) /* minus null */

static unsigned int append_args_str_size(const char **append_args, int append_args_len)
{
    unsigned int i;
    unsigned int len = 0;
    for (i = 0; i < append_args_len; i ++)
        len += strlen(append_args[i]) + 1;
    return len;
}

static int copy_append_args(void __user *dst_addr, const char **append_args, int append_args_len)
{
    unsigned int i;
    unsigned int offset = 0;
    int ret = 0;
    for (i = 0; i < append_args_len; i ++) {
        if (copy_to_user((void __user *)dst_addr + offset, append_args[i],
            strlen(append_args[i]) + 1)) {
            ret = -EINVAL;
            goto out;
        }
        offset += strlen(append_args[i]) + 1;
    }
out:
    return ret;
}

static int copy_between_user_ul(char __user **dst_addr, char __user **src_addr)
{
    char __user *arg;

    if (get_user(arg, src_addr))
        return -EFAULT;

    if (put_user(arg, dst_addr))
        return -EFAULT;

    return 0;
}

static char __user **create_new_args(int argc, char __user **old_argv, const char **append_args, int append_args_len)
{
    int ret;
    unsigned int i;
    unsigned long tmp_addr;
    /* new argv */
    char __user **argv = NULL;
    /* inlcude null since argc doesn't count null */
    unsigned int len = argc + append_args_len + 1;
    unsigned int index = 0, old_index = 0;
    unsigned int offset = 0;
    /* size of the pointer array */
    unsigned int args_size = sizeof(char *) * len;
    /* pointer array + new args size */
    unsigned int mmap_size = args_size + append_args_str_size(append_args, append_args_len);

    argv = (void *)vm_mmap(NULL, 0, mmap_size,
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)argv)) {
        pr_err("mmap failed \n");
        goto out_munmap;
    }

    /* copy append args to memory */
    ret = copy_append_args((void *)argv + args_size, append_args, append_args_len);
    if (ret) {
        pr_err("copy append args failed \n");
        goto out_munmap;
    }

    /* fill pointer array for the basename */
    if (copy_between_user_ul(argv + index, old_argv + old_index)) {
        pr_err("copy args between user failed \n");
        goto out_munmap;
    }

    index ++;
    old_index ++;

    for (i = 0; i < append_args_len; i ++) {
        /* calculate the address of the new pointer array */
        tmp_addr = (unsigned long)argv + args_size + offset;
        /* fill the pointer array */
        if (put_user((char __user *)tmp_addr, argv + index))
            goto out_munmap;
        index ++;
        offset += strlen(append_args[i]) + 1;
    }

    /* copy original parameters */
    while (old_index < argc) {
        if (copy_between_user_ul(argv + index, old_argv + old_index))
            goto out_munmap;
        index ++;
        old_index ++;
    }

    /* add null for the pointer array */
    if (put_user((char __user *)NULL, argv + index))
        goto out_munmap;

    index ++;

    goto out;

out_munmap:
    if (argv && !IS_ERR((void *)argv)) {
        vm_munmap((unsigned long)argv, mmap_size);
        argv = NULL;
    }
out:
    return argv;
}

static int is_compiler_enter_step(const char *name)
{
    if (strncmp(name, CMD_COMPILER_SOURCE_ENTER, STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_COMPILER_PATCHED_ENTER, STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int is_compiler_after_step(const char *name)
{
    if (strncmp(name, CMD_COMPILER_SOURCE_AFTER, STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_COMPILER_PATCHED_AFTER, STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int is_assembler_enter_step(const char *name)
{
    if (strncmp(name, CMD_ASSEMBLER_SOURCE_ENTER, STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_ASSEMBLER_PATCHED_ENTER, STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int is_assembler_after_step(const char *name)
{
    if (strncmp(name, CMD_ASSEMBLER_SOURCE_AFTER, STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_ASSEMBLER_PATCHED_AFTER, STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int setup_envs(char __user **envp, char __user *cmd_addr,
    const char *step_name)
{
    if (is_compiler_enter_step(step_name)) {
        if (put_user((char)'A', (char *)&cmd_addr[COMPILER_CMD_ENV_LEN + 3]))
            return -EFAULT;
    } else if (is_compiler_after_step(step_name)) {
        if (put_user((char)'\0', (char *)&cmd_addr[COMPILER_CMD_ENV_LEN + 1]))
            return -EFAULT;
    } else if (is_assembler_enter_step(step_name)) {
        if (put_user((char)'A', (char *)&cmd_addr[ASSEMBLER_CMD_ENV_LEN + 3]))
            return -EFAULT;
    } else if (is_assembler_after_step(step_name)) {
        if (put_user((char)'\0', (char *)&cmd_addr[ASSEMBLER_CMD_ENV_LEN + 1]))
            return -EFAULT;
    }
    return 0;
}

static char __user *__get_exec_path(void)
{
    int ret;
    char *p;
    struct file *exec_file = NULL;
    char *exec_path = NULL;
    char __user *path_name = NULL;

    exec_path = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!exec_path) {
        pr_err("kmalloc failed \n");
        goto out;
    }

    exec_file = current->mm->exe_file;
    if (!exec_file) {
        pr_err("no exec file found \n");
        goto out;
    }

    p = file_path(exec_file, exec_path, PATH_MAX);
    if (IS_ERR(p)) {
        ret = PTR_ERR(p);
        pr_err("get exec path failed - %d \n", ret);
        goto out;
    }

    memmove(exec_path, p, strlen(p) + 1);

    path_name = (void *)vm_mmap(NULL, 0, PATH_MAX,
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)path_name)) {
        pr_err("mmap path name failed \n");
        goto out;
    }

    if (copy_to_user(path_name, exec_path, PATH_MAX)) {
        pr_err("copy path name to user failed \n");
        vm_munmap((unsigned long)path_name, PATH_MAX);
        path_name = NULL;
        goto out;
    }

out:
    if (exec_path)
        kfree(exec_path);
    return path_name;
}

#define FILENAME_ID_LEN 128
DEFINE_SPINLOCK(filename_lock);
static unsigned long filename_id = 0;

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
    buf[len] = '\0';

    return 0;
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

static int get_assembler_output(char __user **envp, char* out_dir) {
    int ret;
    unsigned long dir_addr;

    ret = obtain_parameter_addr(envp, ASSEMBLER_DIR_ENV, &dir_addr, NULL);
    if (ret || dir_addr == 0) {
        pr_debug("no valid %s \n", ASSEMBLER_DIR_ENV);
        goto out;
    }

    ret = copy_para_from_user((unsigned long)((char *)dir_addr + ASSEMBLER_DIR_ENV_LEN + 1),
        out_dir, PATH_MAX);
    if (ret)
        goto out;
    ret = 0;
out:
    return ret;
}

/*
 * we can build soft link here in three case:
 * 1. file does not exist
 * 2. regular file. this is not allowed: "as -v -o /dev/null /dev/null"
 * 3. file is soft link. there will be a soft link when compiling for the second time
 */
static bool verify_object_path(const char* name)
{
    struct path path;
    struct inode *inode;

    if (kern_path(name, 0, &path))
        return true;

    inode = path.dentry->d_inode;

    if (S_ISREG(inode->i_mode) || S_ISLNK(inode->i_mode))
        return true;

    return false;
}

/*
 * To generate the new object filepath, three ways:
 * 1. append source path from '-o' to the output dir
 * 2. generate new filename based on the HASH mechansim
 * 3. generate new filename using a id number (we use this approach)
 */
static int rewrite_object_path(char __user **argv, const char* out_dir, const char *filename_buff)
{
    int ret;
    size_t len;
    char *object_path = NULL;
    char *kernel_new_path = NULL;
    char *new_path = NULL;
    unsigned long arg_pointer;
    unsigned long arg_addr;

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

    if (!verify_object_path(object_path)) {
        pr_debug("no valid object_path: %s \n", object_path);
        goto out;
    }

    len = strlen(out_dir) + 1 + strlen(filename_buff) + 2 + 1;

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
    strncpy(kernel_new_path + strlen(out_dir) + 1 + strlen(filename_buff), ".o", 2);
    strncpy(kernel_new_path + strlen(out_dir) + 1 + strlen(filename_buff) + 2, "\0", 1);

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
    if (kernel_new_path)
        kfree(kernel_new_path);
    return ret;
}

static char __user **handle_assembler_args(int argc, char __user **argv, char __user **envp)
{
    char filename_buff[FILENAME_ID_LEN];
    char *out_dir = NULL;
    char* buffer = NULL;
    char __user **new_argv = NULL;
    char* assembler_appned_args[] = {
        "--defsym",
        "",
        "NULL"
    };
    int assembler_appned_args_len = (sizeof(assembler_appned_args) / sizeof(char *) - 1);

    out_dir = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!out_dir)
        goto out;

    if (get_assembler_output(envp, out_dir)) {
        pr_debug("get assembler output dir from envp failed! \n");
        goto out;
    }

    if (generate_file_name(filename_buff, FILENAME_ID_LEN)) {
        pr_debug("generate file name failed! \n");
        goto out;
    }

    if (rewrite_object_path(argv, out_dir, filename_buff)) {
        pr_debug("rewrite object path failed! \n");
        goto out;
    }

    buffer = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!buffer)
        goto out;

    snprintf(buffer, PATH_MAX, ".upatch_%s=", filename_buff);
    assembler_appned_args[1] = buffer;
    new_argv = create_new_args(argc, argv, (const char**)assembler_appned_args, assembler_appned_args_len);

out:
    if (out_dir)
        kfree(out_dir);
    if (buffer)
        kfree(buffer);
    return new_argv;
}

static char __user **argv_handler(const char *step_name, int argc, char __user **argv, char __user **envp)
{
    if (is_compiler_enter_step(step_name))
        return create_new_args(argc, argv, compiler_append_args, COMPILER_APPEND_APGS_SIZE);
    else if (is_assembler_enter_step(step_name))
        return handle_assembler_args(argc, argv, envp);
    else
        return NULL;
}

/* return value will be used by uprobe */
int args_handler(struct step *step, struct pt_regs *regs,
    char __user *cmd_addr)
{
    int argc, ret;
    unsigned long stack_pointer = user_stack_pointer(regs);

    char __user **argv = (void *)(stack_pointer + sizeof(unsigned long));
    char __user **envp = NULL;
    char __user *__pathname = NULL;

    if (get_user(argc, (int *)stack_pointer)) {
        pr_err("handler unable to read argc from stack pointer \n");
        /* let uprobe continue to run */
        return exit_syscall(regs, -EINVAL);
    }

    envp = (void *)((unsigned long)argv + (argc + 1) * sizeof(unsigned long));
    ret = setup_envs(envp, cmd_addr, step->name);
    if (ret) {
        pr_err("set up envs failed - %d \n", ret);
        return exit_syscall(regs, ret);
    }

    /* modify args and then execute it again */
    if (is_compiler_enter_step(step->name) || is_assembler_enter_step(step->name)) {
        __pathname = __get_exec_path();
        if (!(const char __user *)__pathname) {
            pr_err("get pathname failed \n");
            return exit_syscall(regs, -EFAULT);
        }

        argv = argv_handler(step->name, argc, argv, envp);
        if (!argv) {
            pr_err("create argv failed \n");
            return exit_syscall(regs, -ENOMEM);
        }

        ret = execve_syscall(regs, __pathname, (void *)argv, (void *)envp);
        if (ret) {
            pr_err("write execve syscall failed with %d \n", ret);
            return 0;
        }

        return UPROBE_ALTER_PC;
    } else if (is_compiler_after_step(step->name) || is_assembler_after_step(step->name)) {
        return 0;
    } else {
        pr_warn("invalid command for upatch compiler \n");
        return 0;
    }
}
