// SPDX-License-Identifier: GPL-2.0-only
/*
 * Copyright (C) 2013 Huawei Ltd.
 * Author: Jiang Liu <liuj97@gmail.com>
 *
 * Copyright (C) 2014-2016 Zi Shen Lim <zlim.lnx@gmail.com>
 */

#ifdef __aarch64__

#include "arch/arm64/insn.h"

#include <asm/debug-monitors.h>

#define ADR_IMM_HILOSPLIT   2
#define ADR_IMM_SIZE        SZ_2M
#define ADR_IMM_LOMASK      ((1 << ADR_IMM_HILOSPLIT) - 1)
#define ADR_IMM_HIMASK      ((ADR_IMM_SIZE >> ADR_IMM_HILOSPLIT) - 1)
#define ADR_IMM_LOSHIFT     29
#define ADR_IMM_HISHIFT     5

static int aarch64_get_imm_shift_mask(enum aarch64_insn_imm_type type, u32 *maskp, int *shiftp)
{
    u32 mask;
    int shift;

    switch (type) {
    case AARCH64_INSN_IMM_26:
        mask = BIT(26) - 1;
        shift = 0;
        break;
    case AARCH64_INSN_IMM_19:
        mask = BIT(19) - 1;
        shift = 5;
        break;
    case AARCH64_INSN_IMM_16:
        mask = BIT(16) - 1;
        shift = 5;
        break;
    case AARCH64_INSN_IMM_14:
        mask = BIT(14) - 1;
        shift = 5;
        break;
    case AARCH64_INSN_IMM_12:
        mask = BIT(12) - 1;
        shift = 10;
        break;
    case AARCH64_INSN_IMM_9:
        mask = BIT(9) - 1;
        shift = 12;
        break;
    case AARCH64_INSN_IMM_7:
        mask = BIT(7) - 1;
        shift = 15;
        break;
    case AARCH64_INSN_IMM_6:
    case AARCH64_INSN_IMM_S:
        mask = BIT(6) - 1;
        shift = 10;
        break;
    case AARCH64_INSN_IMM_R:
        mask = BIT(6) - 1;
        shift = 16;
        break;
    case AARCH64_INSN_IMM_N:
        mask = 1;
        shift = 22;
        break;
    default:
        return -EINVAL;
    }

    *maskp = mask;
    *shiftp = shift;

    return 0;
}

u32 aarch64_insn_encode_immediate(enum aarch64_insn_imm_type type, u32 insn, u64 imm)
{
    u32 immlo, immhi, mask;
    int shift;

    if (insn == AARCH64_BREAK_FAULT)
        return AARCH64_BREAK_FAULT;

    switch (type) {
    case AARCH64_INSN_IMM_ADR:
        shift = 0;
        immlo = (imm & ADR_IMM_LOMASK) << ADR_IMM_LOSHIFT;
        imm >>= ADR_IMM_HILOSPLIT;
        immhi = (imm & ADR_IMM_HIMASK) << ADR_IMM_HISHIFT;
        imm = immlo | immhi;
        mask = ((ADR_IMM_LOMASK << ADR_IMM_LOSHIFT) |
            (ADR_IMM_HIMASK << ADR_IMM_HISHIFT));
        break;
    default:
        if (aarch64_get_imm_shift_mask(type, &mask, &shift) < 0) {
            pr_err("upatch: unknown immediate encoding %d\n",
                   type);
            return AARCH64_BREAK_FAULT;
        }
    }

    /* Update the immediate field. */
    insn &= ~(mask << shift);
    insn |= (imm & mask) << shift;

    return insn;
}

#endif /* __aarch64__ */
