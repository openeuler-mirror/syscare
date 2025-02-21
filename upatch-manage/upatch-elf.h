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

#ifndef __UPATCH_FILE__
#define __UPATCH_FILE__

#include <gelf.h>
#include <stdbool.h>
#include <stdint.h>
#include <unistd.h>

#include "list.h"

#define SYMTAB_NAME ".symtab"
#define DYNSYM_NAME ".dynsym"
#define DYNAMIC_NAME ".dynamic"
#define GOT_RELA_NAME ".rela.dyn"
#define PLT_RELA_NAME ".rela.plt"
#define BUILD_ID_NAME ".note.gnu.build-id"
#define UPATCH_FUNC_NAME ".upatch.funcs"
#define UPATCH_FUNC_STRING ".upatch.strings"
#define TDATA_NAME ".tdata"
#define TBSS_NAME ".tbss"

#define JMP_TABLE_MAX_ENTRY 4096
#define UPATCH_HEADER "UPATCH"
#define UPATCH_HEADER_LEN 6
#define UPATCH_ID_LEN 40

struct upatch_func_addr {
    unsigned long new_addr;
    unsigned long new_size;
    unsigned long old_addr;
    unsigned long old_size;
};

struct upatch_info_func {
    struct upatch_func_addr addr;
    unsigned long old_insn[2];
    unsigned long new_insn;
    char *name;
};

struct upatch_info {
    char magic[7]; // upatch magic
    char id[UPATCH_ID_LEN + 1]; // upatch id
    unsigned long size; // upatch_info and upatch_info_func size
    unsigned long start; // upatch vma start
    unsigned long end; // upatch vma end
    unsigned long changed_func_num;
    struct upatch_info_func *funcs;
    char *func_names;
    unsigned long func_names_size;
};

struct upatch_layout {
    /* The actual code + data. */
    void *kbase;
    void *base;
    /* Total size. */
    unsigned long size;
    /* The size of the executable code.  */
    unsigned long text_size;
    /* Size of RO section of the module (text+rodata) */
    unsigned long ro_size;
    /* Size of RO after init section, not use it now */
    unsigned long ro_after_init_size;
    /* The size of the info.  */
    unsigned long info_size;
};

struct upatch_patch_func {
    struct upatch_func_addr addr;
    unsigned long sympos; /* handle local symbols */
    char *name;
};

struct elf_info {
    const char *name;
    ino_t inode;
    void *patch_buff;
    size_t patch_size;

    GElf_Ehdr *hdr;
    GElf_Shdr *shdrs;
    char *shstrtab;

    unsigned int num_build_id;
    bool is_pie;
    bool is_dyn;
};

struct running_elf {
    struct elf_info info;

    unsigned long num_syms;
    char *strtab;
    char *dynstrtab;

    GElf_Phdr *phdrs;
    GElf_Xword tls_size;
    GElf_Xword tls_align;

    struct {
        unsigned int sym, str;
        unsigned int rela_dyn, rela_plt;
        unsigned int dynsym, dynstr, dynamic;
    } index;

    /* load bias, used to handle ASLR */
    unsigned long load_bias;
    unsigned long load_start;
};

struct upatch_elf {
    struct elf_info info;

    unsigned long num_syms;
    char *strtab;

    struct {
        unsigned int sym, str;
        unsigned int upatch_funcs;
        unsigned int upatch_string;
    } index;

    unsigned long symoffs, stroffs, core_typeoffs;
    unsigned long jmp_offs;
    unsigned int jmp_cur_entry, jmp_max_entry;

    /* memory layout for patch */
    struct upatch_layout core_layout;

    struct running_elf *relf;
};

int upatch_init(struct upatch_elf *, const char *);
int binary_init(struct running_elf *, const char *);
void upatch_close(struct upatch_elf *);
void binary_close(struct running_elf *);

bool is_upatch_section(const char *);

bool is_note_section(GElf_Word);

#endif
