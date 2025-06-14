// SPDX-License-Identifier: GPL-2.0
/*
 * resolve UND symbol in target or VMA so
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

#ifndef _UPATCH_SYMBOL_RESOLVE_H
#define _UPATCH_SYMBOL_RESOLVE_H

#include <linux/types.h>
#include <linux/elf.h>
#include <linux/module.h>

/* This flag appears in a Versym structure.  It means that the symbol
   is hidden, and is only visible with an explicit version number.
   This is a GNU extension.  */
#define VERSYM_HIDDEN       0x8000

#define STT_IFUNC           0xa // when e_ident[EI_OSABI] == ELFOSABI_GNU/ELFOSABI_FREEBSD

#define OK_TYPES (1 << STT_NOTYPE | 1 << STT_OBJECT | 1 << STT_FUNC | 1 << STT_COMMON | 1 << STT_TLS | 1 << STT_IFUNC)
#define OK_BINDS (1 << STB_GLOBAL | 1 << STB_WEAK)

#ifndef SHT_GNU_HASH
#define SHT_GNU_HASH 0x6ffffff6
#endif

#ifndef ELF_BITS
# if ELF_CLASS == ELFCLASS64
#  define ELF_BITS 64
   typedef u64 bloom_t;
# else
#  define ELF_BITS 32
   typedef u32 bloom_t;
# endif
#endif

struct running_elf;

unsigned long resolve_symbol(const struct running_elf *relf, const char *name, Elf_Sym patch_sym);

#endif // _UPATCH_SYMBOL_RESOLVE_H
