// SPDX-License-Identifier: GPL-2.0
/*
 * when user program hit uprobe trap and go into kernel, load patch into VMA
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

#ifndef _UPATCH_IOCTL_PATCH_LOAD_H
#define _UPATCH_IOCTL_PATCH_LOAD_H

#include <linux/elf.h>
#include <linux/module.h>

struct target_entity;
struct target_metadata;
struct patch_entity;
struct process_entity;

struct upatch_info;

struct jmp_table {
    unsigned long off;
    unsigned int cur;
    unsigned int max;
};

/* memory layout for module */
struct upatch_layout {
    void* kbase;                // kmalloc patch, will be relocated and copy to base
    unsigned long base;         // VMA in user space
    unsigned int size;          // Total size
    unsigned int text_end;      // The size of the executable code and jmp table
    unsigned int ro_end;        // Size of RO section of the module (text+rodata)
    unsigned int ro_after_init_end; // Size of RO after init section
    struct jmp_table table;
};

struct running_elf {
    struct target_metadata *meta;

    // target vma start addr, the first vma could not be the text in LLVM
    unsigned long vma_start_addr;

    struct upatch_info *load_info;
};

// when load patch, patch need resolve in different process
struct upatch_info {
    unsigned long len;
    Elf_Ehdr *ehdr;
    Elf_Shdr *shdrs;
    Elf_Shdr *upatch_func_sec;
    char *shshdrtab, *strtab;
    unsigned int und_cnt, got_rela_cnt;
    struct {
        unsigned int sym, str;
    } index;

    /* memory layout for patch */
    struct upatch_layout layout;

    struct running_elf running_elf;
};

int upatch_resolve(struct target_entity *target, struct patch_entity *patch, struct process_entity *process,
    unsigned long target_code_start);

bool is_got_rela_type(int type);

// All UND symbol have already been set up got table in resolve_symbol.c
// Except thoese GLOBAL OBJECT in target found in resolve_from_target_sym
unsigned long get_or_setup_got_entry(struct upatch_info *info, Elf_Sym *sym);

#endif // _UPATCH_IOCTL_PATCH_LOAD_H
