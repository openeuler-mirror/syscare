// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifndef _ARCH_PATCH_SYSCALL_H
#define _ARCH_PATCH_SYSCALL_H

#include <linux/ptrace.h>

asmlinkage int __execve_syscall(struct pt_regs *regs, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp);
asmlinkage int __exit_syscall(struct pt_regs *regs, int exit_code);

void set_execve_syscall_registers(struct pt_regs *regs, const void __user *code, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp);
void set_exit_syscall_registers(struct pt_regs *regs, const void __user *code, int exit_code);

#endif /* _ARCH_PATCH_SYSCALL_H */
