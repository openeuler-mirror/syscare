// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-lib
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

#ifndef __UPATCH_RESOLVE_H_
#define __UPATCH_RESOLVE_H_

#include "list.h"
#include "upatch-elf.h"

struct list_head* patch_symbols_resolve(struct upatch_elf *uelf, struct running_elf *relf);
void patch_symbols_free(struct list_head *symbols);

#endif
