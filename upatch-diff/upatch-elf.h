// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-elf.h
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

#ifndef __UPATCH_ELF_H_
#define __UPATCH_ELF_H_

#include <stdbool.h>
#include <gelf.h>

#include "list.h"

extern char *g_relf_name;

// these data structs contain each other
struct section;
struct rela;
struct symbol;

enum architecture {
    X86_64  = 0x1 << 0,
    AARCH64 = 0x1 << 1,
    RISCV64 = 0x1 << 2,
};

enum data_source {
    DATA_SOURCE_REF,
    DATA_SOURCE_ALLOC,
};

enum status {
    SAME,
    CHANGED,
    NEW,
};

struct upatch_elf {
    int fd;
    Elf *elf;
    enum architecture arch;
    struct list_head sections;
    struct list_head symbols;
    struct list_head strings;
};

struct section {
    struct list_head list;
    GElf_Shdr sh;

    GElf_Section index;
    char *name;
    Elf_Data *data;

    /* data source */
    enum data_source name_source;
    enum data_source data_source;
    enum data_source dbuf_source;

    /* section info */
    struct section *link;
    void *info;

    /* symbol reference */
    struct symbol *sym;
    struct symbol *bundle_sym;

    /* reloc reference */
    struct section *base;
    struct section *rela;
    struct list_head relas;

    /* diff metadata */
    struct section *twin;
    enum status status;
    bool grouped;
    bool ignored;
    bool include;
};

struct symbol {
    struct list_head list;
    GElf_Sym sym;

    GElf_Word index;
    char *name;

    /* data source */
    enum data_source name_source;

    /* symbol info */
    unsigned char bind;
    unsigned char type;

    /* section reference */
    struct section *sec;

    /* subfunction reference */
    struct symbol *parent;
    struct list_head children;
    struct list_head subfunction_node;

    /* diff metadata */
    struct symbol *twin;
    enum status status;
    bool include; /* used in the patched elf */
    bool strip; /* used in the output elf */
};

struct rela {
    struct list_head list;
    GElf_Rela rela;

    /* symbol reference */
    struct symbol *sym;

    /* rela info */
    GElf_Word type;
    GElf_Off offset;
    GElf_Sxword addend;

    char *string;
};

struct string {
    struct list_head list;
    char *name;
};

void uelf_open(struct upatch_elf *uelf, const char *name);
void uelf_close(struct upatch_elf *uelf);

#endif
