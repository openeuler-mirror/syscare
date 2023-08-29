// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifdef __x86_64__

#include "arch/patch-syscall.h"

void set_execve_syscall_registers(struct pt_regs *regs, const void __user *code, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp)
{
    regs->ip = (unsigned long)code;
    regs->di = (unsigned long)pathname;
    regs->si = (unsigned long)argv;
    regs->dx = (unsigned long)envp;
}

void set_exit_syscall_registers(struct pt_regs *regs, const void __user *code, int exit_code)
{
    regs->ip = (unsigned long)code;
    regs->di = exit_code;
}

#endif /* __x86_64__ */
