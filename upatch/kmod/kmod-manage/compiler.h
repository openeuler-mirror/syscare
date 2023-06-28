// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *   Zongwu Li <lzw32321226@163.com>
 *
 */

#ifndef _UPATCH_COMPILER_H
#define _UPATCH_COMPILER_H

#include "compiler-args.h"

int compiler_hack_init(void);
void compiler_hack_exit(void);

int register_compiler_step(char *, step_handler_t);
void unregister_compiler_step(char *);

int register_assembler_step(char *, step_handler_t);
void unregister_assembler_step(char *);

void clear_compiler_step(void);
void clear_assembler_step(void);

int handle_compiler_cmd(unsigned long, unsigned int);

#endif /* _UPATCH_COMPILER_H */