// SPDX-License-Identifier: GPL-2.0
/*
 * maintain patch info header
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

#ifndef _UPATCH_MANAGE_PATCH_ENTITY_H
#define _UPATCH_MANAGE_PATCH_ENTITY_H

#include <linux/types.h>
#include <linux/elf.h>
#include <linux/module.h>
#include <linux/kref.h>

struct file;
struct inode;
struct target_entity;

/* Patch status */
enum upatch_status {
    UPATCH_STATUS_NOT_APPLIED = 1,
    UPATCH_STATUS_DEACTIVED,
    UPATCH_STATUS_ACTIVED
};

static inline const char *patch_status_str(int status)
{
    static const char *STATUS_STR[] = {"NOT_APPLIED", "DEACTIVED", "ACTIVED"};

    if (status < UPATCH_STATUS_NOT_APPLIED || status > UPATCH_STATUS_ACTIVED) {
        return "UNKNOWN";
    }

    return STATUS_STR[status - 1];
}

/* Patch function relocation */
struct upatch_relocation {
    Elf_Rela addr;
    Elf_Rela name;
};

#ifdef CONFIG_64BIT
 /* Patch function entity */
struct upatch_function {
    u64 new_addr;
    u64 new_size;
    u64 old_addr;
    u64 old_size;
    u64 sympos;     // handle local symbols
    u64 name_off;   // name offset in .upatch.strings
};
#else
/* Patch function entity */
struct upatch_function {
    u32 new_addr;
    u32 new_size;
    u32 old_addr;
    u32 old_size;
    u32 sympos;     // handle local symbols
    u32 name_off;   // name offset in .upatch.strings
    u32 padding1;
    u32 padding2;
};
#endif

/* Patch file */
struct patch_file {
    char path_buff[PATH_MAX];

    const char *path;               // patch file path
    struct inode *inode;            // patch file path
    loff_t size;                    // patch file size

    void *buff;                     // patch file buff

    Elf_Half shstrtab_index;        // section '.shstrtab' index
    Elf_Half symtab_index;          // section '.symtab' index
    Elf_Half strtab_index;          // section '.strtab' index

    Elf_Half func_index;            // section '.upatch.funcs' index
    Elf_Half rela_index;            // section '.rela.upatch.funcs' index
    Elf_Half string_index;          // section '.upatch.strings' index

    struct upatch_function *funcs;  // patch function table
    const char *strings;            // patch string table

    size_t func_num;                // patch function count
    size_t string_len;              // patch string table length

    size_t und_sym_num;             // undefined symbol count (SHN_UNDEF)
    size_t got_reloc_num;           // got relocation count
};

/* Patch entity */
struct patch_entity {
    struct patch_file file;         // patch file
    enum upatch_status status;      // patch status

    struct hlist_node node;         // hash table node
    struct list_head actived_node;  // target actived patch list node

    struct kref kref;
};

/**
 * @brief Load a new patch file
 * @param file: Patch file struct pointer
 * @return Newly allocated patch entity with refcount=1, or NULL on failure
 *
 * Allocates and initializes a new patch entity structure with reference count 1.
 * The caller is responsible for calling put_patch() when done.
 */
struct patch_entity *load_patch(struct file *file);

/**
 * @brief Release patch resources when refcount reaches zero
 * @param kref: Reference counter
 *
 * Called automatically by kref_put().
 * Frees all patch resources and disassociates from target.
 */
void release_patch(struct kref *kref);

/**
 * @brief Acquire a reference to a patch entity
 * @param patch: Patch entity pointer
 * @return Patch with incremented refcount, or NULL if input is NULL
 *
 * Caller must balance with put_patch().
 */
static inline struct patch_entity *get_patch(struct patch_entity *patch)
{
    if (unlikely(!patch)) {
        return NULL;
    }

    kref_get(&patch->kref);
    return patch;
}

/**
 * @brief Release a patch entity reference
 * @param patch: Patch entity pointer
 *
 * Decrements refcount and triggers release_patch_entity() when reaching zero.
 * Safe to call with NULL.
 */
static inline void put_patch(struct patch_entity *patch)
{
    if (unlikely(!patch)) {
        return;
    }

    kref_put(&patch->kref, release_patch);
}

#endif // _UPATCH_MANAGE_PATCH_ENTITY_H
