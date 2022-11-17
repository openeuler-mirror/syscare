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

#include <asm/uaccess.h>
#include <asm/ptrace.h>

#include "compiler-args.h"
#include "asm/hijack.h"

static const char *append_args[] = {
    "-gdwarf", /* obatain debug information */
    "-ffunction-sections",
    "-fno-asynchronous-unwind-tables",
    "-fdata-sections",
    "NULL",
};

#define APPEND_APGS_SIZE (sizeof(append_args) / sizeof(char *) - 1) /* minus null */

static unsigned int append_args_str_size(void)
{
    unsigned int i;
    unsigned int len = 0;
    for (i = 0; i < APPEND_APGS_SIZE; i ++)
        len += strlen(append_args[i]) + 1;
    return len;
}

static int copy_append_args(void __user *dst_addr)
{
    unsigned int i;
    unsigned int offset = 0;
    int ret = 0;
    for (i = 0; i < APPEND_APGS_SIZE; i ++) {
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

static char __user **create_new_args(int argc, char __user **old_argv)
{
    int ret;
    unsigned int i;
    unsigned long tmp_addr;
    /* new argv */
    char __user **argv = NULL;
    /* inlcude null since argc doesn't count null */
    unsigned int len = argc + APPEND_APGS_SIZE + 1;
    unsigned int index = 0, old_index = 0;
    unsigned int offset = 0;
    /* size of the pointer array */
    unsigned int args_size = sizeof(char *) * len;
    /* pointer array + new args size */
    unsigned int mmap_size = args_size + append_args_str_size();

    argv = (void *)vm_mmap(NULL, 0, mmap_size,
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)argv)) {
        pr_err("mmap failed \n");
        goto out_munmap;
    }

    /* copy append args to memory */
    ret = copy_append_args((void *)argv + args_size);
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

    /* for gcc: gcc [options] file ... */
    for (i = 0; i < APPEND_APGS_SIZE; i ++) {
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
    if (!IS_ERR((void *)argv)) {
        vm_munmap((unsigned long)argv, mmap_size);
        argv = NULL;
    }
out:
    return argv;
}

static int is_enter_step(const char *name)
{
    if (strncmp(name, CMD_SOURCE_ENTER, COMPILER_STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_PATCHED_ENTER, COMPILER_STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int is_after_step(const char *name)
{
    if (strncmp(name, CMD_SOURCE_AFTER, COMPILER_STEP_MAX_LEN) == 0 ||
        strncmp(name, CMD_PATCHED_AFTER, COMPILER_STEP_MAX_LEN) == 0)
        return 1;
    return 0;
}

static int setup_envs(char __user **envp, char __user *cmd_addr,
    const char *step_name)
{
    if (is_enter_step(step_name)) {
        if (put_user((char)'A', (char *)&cmd_addr[COMPILER_CMD_ENV_LEN + 2]))
            return -EFAULT;
    } else if (is_after_step(step_name)) {
        if (put_user((char)'\0', (char *)&cmd_addr[COMPILER_CMD_ENV_LEN + 1]))
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

/* return value will be used by uprobe */
int compiler_args_handler(struct compiler_step *step, struct pt_regs *regs,
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
        return run_exit_syscall(regs, -EINVAL);;
    }

    envp = (void *)((unsigned long)argv + (argc + 1) * sizeof(unsigned long));
    ret = setup_envs(envp, cmd_addr, step->name);
    if (ret) {
        pr_err("set up envs failed - %d \n", ret);
        return run_exit_syscall(regs, ret);;
    }

    /* modify args and then execute it again */
    if (is_enter_step(step->name)) {
        __pathname = __get_exec_path();
        if (!(const char __user *)__pathname) {
            pr_err("get pathname failed \n");
            return run_exit_syscall(regs, -EFAULT);
        }

        argv = create_new_args(argc, argv);
        if (!argv) {
            pr_err("create argv failed \n");
            return run_exit_syscall(regs, -ENOMEM);
        }

        ret = run_execve_syscall(regs, __pathname, (void *)argv, (void *)envp);
        if (ret) {
            pr_err("write execve syscall failed with %d \n", ret);
            return 0;
        }

        return UPROBE_ALTER_PC;
    } else if (is_after_step(step->name)) {
        return 0;
    } else {
        pr_warn("invalid command for upatch compiler \n");
        return 0;
    }
}
