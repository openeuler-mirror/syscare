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

#ifndef __UPATCH_RESOLVE__
#define __UPATCH_RESOLVE__

#include "upatch-elf.h"
#include "upatch-process.h"

#define SHN_LIVEPATCH 0xff20

/* jmp table, solve limit for the jmp instruction, Used for both PLT/GOT */
struct upatch_jmp_table_entry;

unsigned int get_jmp_table_entry(void);

unsigned long insert_plt_table(struct upatch_elf *, struct object_file *,
			       unsigned long, unsigned long);
unsigned long insert_got_table(struct upatch_elf *, struct object_file *,
			       unsigned long, unsigned long);

unsigned long search_insert_plt_table(struct upatch_elf *, unsigned long,
				      unsigned long);

int simplify_symbols(struct upatch_elf *, struct object_file *);

#endif