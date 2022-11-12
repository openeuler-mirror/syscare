// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_HIJACK_H
#define _UPATCH_HIJACK_H

#include <asm/ptrace.h>

extern int run_execve_syscall(struct pt_regs *regs, const char __user *pathname,
    const char __user *const __user *argv, const char __user *const __user *envp);

extern int run_exit_syscall(struct pt_regs *regs, int exit_val);

#endif /* _UPATCH_HIJACK_H */


