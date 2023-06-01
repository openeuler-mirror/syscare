// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifndef _ARCH_PATCH_LOAD_H
#define _ARCH_PATCH_LOAD_H

#include <asm/elf.h>

#include "patch.h"

struct upatch_load_info;
struct upatch_module;

/* jmp table, solve limit for the jmp instruction */
#if defined(__x86_64__)
struct upatch_jmp_table_entry {
    unsigned long inst;
    unsigned long addr;
};
#elif defined(__aarch64__)
struct upatch_jmp_table_entry {
    unsigned long inst[2];
    unsigned long addr[2];
};
#endif

/* Used for both PLT/GOT */
unsigned long setup_jmp_table(struct upatch_load_info *info, unsigned long jmp_addr,
    unsigned long tmp_addr);

int apply_relocate_add(struct upatch_load_info *info, Elf64_Shdr *sechdrs,
    const char *strtab, unsigned int symindex,
    unsigned int relsec, struct upatch_module *me);

void setup_parameters(struct pt_regs *regs, unsigned long para_a,
    unsigned long para_b, unsigned long para_c);

#endif /* _ARCH_PATCH_LOAD_H */
