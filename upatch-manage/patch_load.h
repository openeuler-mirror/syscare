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
struct patch_metadata;

struct process_entity;

struct jmp_table {
    unsigned long off;
    unsigned int cur;
    unsigned int max;
};

/* memory layout for module */
struct patch_layout {
    void* kbase;                    // patch image in kernelspace
    unsigned long base;             // patch in userspace
    unsigned int size;              // patch total size
    unsigned int text_end;          // patch executable code & jump table size
    unsigned int ro_end;            // patch read-only section size (text+rodata)
    unsigned int ro_after_init_end; // patch read-only after init section size
    struct jmp_table table;
};

// when load patch, patch need resolve in different process
struct patch_context {
    const struct target_metadata *target;
    const struct patch_metadata *patch;
    unsigned long load_bias;

    void *buff; // patch image in kernelspace
    struct patch_layout layout;

    Elf_Ehdr *ehdr;
    Elf_Shdr *shdrs;

    Elf_Shdr *shstrtab_shdr;
    Elf_Shdr *symtab_shdr;
    Elf_Shdr *strtab_shdr;

    Elf_Shdr *func_shdr;
    Elf_Shdr *rela_shdr;
    Elf_Shdr *string_shdr;

    void *plt;
    uintptr_t *got;
};

int upatch_resolve(struct target_entity *target, struct patch_entity *patch, struct process_entity *process,
    unsigned long vma_start);

bool is_got_rela_type(int type);

// All UND symbol have already been set up got table in resolve_symbol.c
// Except thoese GLOBAL OBJECT in target found in resolve_from_target_sym
unsigned long get_or_setup_got_entry(struct patch_context *ctx, Elf_Sym *sym);

#endif // _UPATCH_IOCTL_PATCH_LOAD_H
