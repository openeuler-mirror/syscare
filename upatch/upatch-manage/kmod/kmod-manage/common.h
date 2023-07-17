// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_COMMON_H
#define _UPATCH_COMMON_H

#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/slab.h>
#include <linux/elf.h>
#include <linux/mm.h>

/* Common used tool functions */
int copy_para_from_user(unsigned long, char *, size_t);

struct file *get_binary_file_from_addr(struct task_struct *, unsigned long);

static bool inline streql(const char *a, const char *b)
{
    return strlen(a) == strlen(b) && !strncmp(a, b, strlen(a));
}

int obtain_parameter_addr(char __user **, char *, unsigned long *, unsigned long *);

int obtain_parameter_pointer(char __user **, char *, unsigned long *, unsigned long *);

char __user **get_argv_from_regs(struct pt_regs *);

char __user **get_env_from_regs(struct pt_regs *);

#endif /* _UPATCH_COMMON_H */
