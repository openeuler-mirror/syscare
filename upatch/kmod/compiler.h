// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_COMPILER_H
#define _UPATCH_COMPILER_H

/* env variables for UPATCH */
#define COMPILER_CMD_ENV "UPATCH_CMD"
/* strlen(COMPILER_CMD_ENV) */
#define COMPILER_CMD_ENV_LEN 10
#define COMPILER_STEP_MAX_LEN 8
/* COMPILER_CMD_ENV_LEN + '=' + COMPILER_STEP_MAX_LEN */
#define COMPILER_CMD_MAX_LEN 32

#define ASSEMBLER_DIR_ENV "UPATCH_OUTPUT"
#define ASSEMBLER_DIR_ENV_LEN 13

struct compiler_step;
typedef int (*cs_handler_t)(struct compiler_step *step,
    struct pt_regs *regs, char __user *cmd_addr);
struct compiler_step {
    char name[COMPILER_STEP_MAX_LEN];
    cs_handler_t step_handler;
    struct list_head list;
};

int compiler_hack_init(void);
void compiler_hack_exit(void);

int register_compiler_step(char *name, cs_handler_t step_handler);
void unregister_compiler_step(char *name);

void clear_compiler_step(void);

#endif /* _UPATCH_COMPILER_H */