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
#include <linux/list.h>

#include <linux/elf.h>
#include <linux/module.h>

struct inode;
struct target_entity;

#define PATCHES_HASH_BITS 4

/* Patch status */
enum upatch_status {
    UPATCH_STATUS_NOT_APPLIED = 1,
    UPATCH_STATUS_DEACTIVED,
    UPATCH_STATUS_ACTIVED
};

static inline const char *patch_status(int status)
{
    static const char *STATUS_STR[] = {"NOT_APPLIED", "DEACTIVED", "ACTIVED"};

    if (status < UPATCH_STATUS_NOT_APPLIED || status > UPATCH_STATUS_ACTIVED) {
        return "INVALID_STATUS";
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

/* Patch metadata */
struct upatch_metadata {
    struct upatch_function *funcs;   // this should vmalloc, if not, relocation of new_addr may fail
    struct upatch_relocation *relas; // .rela.upatch.funcs
    char *strings;                   // .upatch.strings

    struct {
        unsigned int sym, str;
    } index;

    size_t func_count;
    size_t und_count;                // UND symbol count
    size_t got_rela_cnt;             // relocation type need add got table cnt

    void *patch_buff;
    size_t patch_size;
};

/* Patch entity */
struct patch_entity {
    char *path;                     // patch file path
    struct inode *inode;            // patch file inode

    struct upatch_metadata meta;    // patch metadata
    struct target_entity *target;   // target file inode
    enum upatch_status status;      // patch status

    struct hlist_node node;         // all patches store in hash table
    struct list_head patch_node;    // patch node in target entity
    struct list_head actived_node;  // actived patch in target entity
};

struct patch_entity *get_patch_entity(const char *patch_file);

struct patch_entity *new_patch_entity(const char *patch_file);

void free_patch_entity(struct patch_entity *patch);

void __exit verify_patch_empty_on_exit(void);

#endif // _UPATCH_MANAGE_PATCH_ENTITY_H
