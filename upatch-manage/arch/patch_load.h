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

#include "../patch_load.h"
#include "../patch_entity.h"
#include "../target_entity.h"
#include "../util.h"

/* jmp table, solve limit for the jmp instruction, Used for both PLT/GOT */
#if defined(__aarch64__)
struct upatch_jmp_table_entry {
    unsigned long inst[2];
    unsigned long addr[2];
};
#else
struct upatch_jmp_table_entry {
    unsigned long inst;
    unsigned long addr;
};
#endif

#define JMP_ENTRY_SIZE (sizeof(unsigned long))
#if defined(__arm__)
#define NORMAL_JMP_ENTRY_NUM 3
#else
#define NORMAL_JMP_ENTRY_NUM 2
#endif

#define JMP_TABLE_GOT_ENTRY_SIZE (JMP_ENTRY_SIZE * NORMAL_JMP_ENTRY_NUM)

#if defined(__x86_64__)
#define IFUNC_JMP_ENTRY_NUM 3
// R_X86_64_PC32 uses a 32-bit signed offset, allowing a +/- 2 GiB range.
#define PATCH_LOAD_RANGE_LIMIT (1UL << 31) // 2 GiB (2^31)

#elif defined(__aarch64__)
#define IFUNC_JMP_ENTRY_NUM 5
#define PLT_JMP_ENTRY_NUM 4
// R_AARCH64_JUMP26/CALL26 uses a 26-bit immediate, shifted left by 2, signed -> +/- 128 MiB range.
#define PATCH_LOAD_RANGE_LIMIT (1UL << 27) // 128 MiB (2^27)

#elif defined(__arm__)
#define IFUNC_JMP_ENTRY_NUM 10
// R_ARM_JUMP24/CALL uses a 24-bit immediate, shifted left by 2, signed -> +/- 32 MiB range.
#define PATCH_LOAD_RANGE_LIMIT (1UL << 25) // 32 MiB (2^25)
#endif

#define JMP_TABLE_ENTRY_MAX_SIZE (JMP_ENTRY_SIZE * IFUNC_JMP_ENTRY_NUM)

unsigned long insert_plt_table(struct patch_context *ctx, unsigned long r_type, void __user *addr);

// write jmp addr in jmp table in text section, return the real jmp entry address in VMA
unsigned long setup_jmp_table(struct patch_context *ctx, unsigned long jmp_addr, bool is_ifunc);

unsigned long insert_got_table(struct patch_context *ctx, unsigned long r_type, void __user *addr);

unsigned long setup_got_table(struct patch_context *ctx, unsigned long jmp_addr, unsigned long tls_addr);

int apply_relocate_add(struct patch_context *ctx, unsigned int relsec);

#endif /* _ARCH_PATCH_LOAD_H */
