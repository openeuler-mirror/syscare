// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_COMPILER_ARGS_H
#define _UPATCH_COMPILER_ARGS_H

#include "compiler.h"

#define CMD_SOURCE_ENTER   "SE"
#define CMD_SOURCE_AFTER  "SA"

#define CMD_PATCHED_ENTER  "PE"
#define CMD_PATCHED_AFTER "PA"

int compiler_args_handler(struct compiler_step *step, struct pt_regs *regs,
    char __user *cmd_addr);

#endif /* _UPATCH_COMPILER_ARGS_H */


