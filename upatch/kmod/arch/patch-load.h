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

unsigned long jmp_table_inst(void);

int apply_relocate_add(struct upatch_load_info *info, Elf64_Shdr *sechdrs,
    const char *strtab, unsigned int symindex,
    unsigned int relsec, struct upatch_module *me);

#endif /* _ARCH_PATCH_LOAD_H */
