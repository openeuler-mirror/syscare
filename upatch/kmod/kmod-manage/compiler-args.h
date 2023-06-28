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

#include <linux/types.h>

// for gcc
#define CMD_COMPILER_SOURCE_ENTER   "CSE"
#define CMD_COMPILER_SOURCE_AFTER   "CSA"

#define CMD_COMPILER_PATCHED_ENTER  "CPE"
#define CMD_COMPILER_PATCHED_AFTER  "CPA"

/* env variables for UPATCH */
#define COMPILER_CMD_ENV "UPATCH_COMPILER_CMD"
/* strlen(COMPILER_CMD_ENV) */
#define COMPILER_CMD_ENV_LEN 19
#define STEP_MAX_LEN 3

struct step;
typedef int (*step_handler_t)(struct step *step,
    struct pt_regs *regs, char __user *cmd_addr);
struct step {
    char name[STEP_MAX_LEN];
    step_handler_t step_handler;
    struct list_head list;
};

int args_handler(struct step *step, struct pt_regs *regs,
    char __user *cmd_addr);

// for assembler
#define CMD_ASSEMBLER_SOURCE_ENTER  "ASE"
#define CMD_ASSEMBLER_SOURCE_AFTER  "ASA"

#define CMD_ASSEMBLER_PATCHED_ENTER "APE"
#define CMD_ASSEMBLER_PATCHED_AFTER "APA"
/* env variables for UPATCH */
#define ASSEMBLER_CMD_ENV "UPATCH_ASSEMBLER_CMD"
/* strlen(COMPILER_CMD_ENV) */
#define ASSEMBLER_CMD_ENV_LEN 20
/* COMPILER_CMD_ENV_LEN/ASSEMBLER_CMD_ENV_LEN + '=' + STEP_MAX_LEN */
#define CMD_MAX_LEN 26

#define ASSEMBLER_DIR_ENV "UPATCH_ASSEMBLER_OUTPUT"
#define ASSEMBLER_DIR_ENV_LEN 23

#endif /* _UPATCH_COMPILER_ARGS_H */
