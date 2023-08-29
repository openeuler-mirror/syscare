// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifndef _PATCH_SYSCALL_H
#define _PATCH_SYSCALL_H

#include <linux/ptrace.h>

int execve_syscall(struct pt_regs *regs, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp);

int exit_syscall(struct pt_regs *regs, int exit_val);

#endif /* _PATCH_SYSCALL_H */
