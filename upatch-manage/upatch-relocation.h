// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
 * Copyright (C) 2024 Huawei Technologies Co., Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#ifndef __UPATCH_RELOCATION__
#define __UPATCH_RELOCATION__

#include <gelf.h>

#include "log.h"
#include "upatch-common.h"
#include "upatch-elf.h"

// TODO: change define
#define s64 int64_t
#define u64 uint64_t
#define u32 uint32_t
#define s32 int32_t
#define u16 uint16_t
#define s16 int16_t

int apply_relocate_add(struct upatch_elf *, unsigned int, unsigned int);

int apply_relocations(struct upatch_elf *);

#endif
