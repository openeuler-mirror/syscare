// SPDX-License-Identifier: GPL-2.0-only
/*
 * Copyright (C) 2013 Huawei Ltd.
 * Author: Jiang Liu <liuj97@gmail.com>
 *
 * Copyright (C) 2014-2016 Zi Shen Lim <zlim.lnx@gmail.com>
 */

#ifndef _ARCH_AARCH64_INSN_H
#define _ARCH_AARCH64_INSN_H

#include <asm/insn.h>

u32 aarch64_insn_encode_immediate(enum aarch64_insn_imm_type type, u32 insn, u64 imm);

#endif /* _ARCH_AARCH64_INSN_H */
