// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifdef __aarch64__

#include "arch/patch-syscall.h"

void set_execve_syscall_registers(struct pt_regs *regs, const void __user *code,
    const char __user *pathname, const char __user *const __user *argv, const char __user *const __user *envp)
{
    regs->pc = (unsigned long)code;
    regs->regs[0] = (unsigned long)pathname;
    regs->regs[1] = (unsigned long)argv;
    regs->regs[2] = (unsigned long)envp;
}

void set_exit_syscall_registers(struct pt_regs *regs, const void __user *code, int exit_code)
{
    regs->pc = (unsigned long)code;
    regs->regs[0] = exit_code;
}

#endif /* __aarch64__ */
