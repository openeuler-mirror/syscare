// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/mman.h>
#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/highmem.h>
#include <linux/vmalloc.h>

/* functions from hijack_code.S */
#define HIJACK_MAX_LEN PAGE_SIZE
extern void __run_execve(void);
extern void __run_exit(int);

static void __user *set_code_buffer(void *code, size_t len)
{
    void __user *code_buffer = NULL;

    code_buffer = (char __user *)vm_mmap(NULL, 0, len,
        PROT_READ | PROT_WRITE | PROT_EXEC, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR(code_buffer)) {
        pr_err("alloc user memory failed \n");
        goto out;
    }

    if (copy_to_user(code_buffer, code, len)) {
        pr_err("copy to user memory failed \n");
        vm_munmap((unsigned long)code_buffer, len);
        goto out;
    }

out:
    return code_buffer;
}

int run_execve_syscall(struct pt_regs *regs, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp)
{
    void __user *code_buffer = NULL;

    code_buffer = set_code_buffer(__run_execve, HIJACK_MAX_LEN);
    if (!(const char __user *)code_buffer)
        return -EFAULT;

    /* set parameters for syscall */
    regs->di = (unsigned long)pathname;
    regs->si = (unsigned long)argv;
    regs->dx = (unsigned long)envp;

    instruction_pointer_set(regs, (unsigned long)code_buffer);

    return 0;
}

int run_exit_syscall(struct pt_regs *regs, int exit_val)
{
    void __user *code_buffer = NULL;

    code_buffer = set_code_buffer(__run_exit, HIJACK_MAX_LEN);
    if (!(const char __user *)code_buffer)
        return -EFAULT;

    regs->di = exit_val;

    instruction_pointer_set(regs, (unsigned long)code_buffer);

    return 0;
}