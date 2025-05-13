// SPDX-License-Identifier: GPL-2.0
/*
 * running-elf.h
 *
 * Copyright (C) 2014 Seth Jennings <sjenning@redhat.com>
 * Copyright (C) 2013-2014 Josh Poimboeuf <jpoimboe@redhat.com>
 * Copyright (C) 2022 Longjun Luo <luolongjun@huawei.com>
 * Copyright (C) 2022 Zongwu Li <lizongwu@huawei.com>
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA,
 * 02110-1301, USA.
 */

#ifndef __RUNNING_ELF_H_
#define __RUNNING_ELF_H_

#include <stdbool.h>
#include <errno.h>
#include <gelf.h>

struct symbol;

struct relf_symbol {
    GElf_Word index;
    char *name;
    unsigned char type;
    unsigned char bind;
    GElf_Section shndx;
    GElf_Addr addr;
    GElf_Xword size;
};

struct running_elf {
    int fd;
    Elf *elf;
    struct relf_symbol *symbols;
    GElf_Word symbol_count;
    bool is_exec;
};

void relf_open(struct running_elf *relf, const char *name);
void relf_close(struct running_elf *relf);

struct relf_symbol* lookup_relf(struct running_elf *relf, struct symbol *sym);

#endif
