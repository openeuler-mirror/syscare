// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/linkage.h>
#include <linux/mman.h>

#include "arch/patch-syscall.h"
#include "patch-syscall.h"

#define MAX_CODE_SIZE PAGE_SIZE

static void __user *alloc_code_mem(void *code, size_t len)
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

int execve_syscall(struct pt_regs *regs, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp)
{
    void __user *code = alloc_code_mem(__execve_syscall, MAX_CODE_SIZE);
    if (!(const char __user *)code)
        return -EFAULT;

    set_execve_syscall_registers(regs, code, pathname, argv, envp);

    return 0;
}

int exit_syscall(struct pt_regs *regs, int exit_code)
{
    void __user *code = alloc_code_mem(__exit_syscall, MAX_CODE_SIZE);
    if (!(const char __user *)code)
        return -EFAULT;

    set_exit_syscall_registers(regs, code, exit_code);

    return 0;
}
