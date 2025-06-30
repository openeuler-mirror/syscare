// SPDX-License-Identifier: GPL-2.0
/*
 * maintain info about the target binary file like executive or shared object
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

#ifndef _UPATCH_MANAGE_TARGET_ENTITY_H
#define _UPATCH_MANAGE_TARGET_ENTITY_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/rwsem.h>
#include <linux/mutex.h>

#include <linux/elf.h>
#include <linux/module.h>

#if defined(__x86_64__)
    #if defined(__CET__) || defined(__SHSTK__)
        #define PLT_ENTRY_SIZE  20 // PLT with CET
    #else
        #define PLT_ENTRY_SIZE  16
    #endif
#elif defined(__aarch64__)
    #if defined(__ARM_PAC) || defined(__ARM_FEATURE_PAC_DEFAULT)
        #define PLT_ENTRY_SIZE  20 // PLT with PAC
    #else
        #define PLT_ENTRY_SIZE  16
    #endif
#endif
#define GOT_ENTRY_SIZE  sizeof(uintptr_t)  // GOT entry size is pointer size

struct inode;

struct patch_entity;
struct upatch_function;

/* target elf metadata */
struct target_metadata {
    const char *path;
    struct inode *inode;

    loff_t size;

    Elf_Ehdr *ehdr;
    Elf_Phdr *phdrs;
    Elf_Shdr *shdrs;

    Elf_Sym *symtab;
    Elf_Sym *dynsym;
    Elf_Dyn *dynamic;
    Elf_Rela *rela_dyn;
    Elf_Rela *rela_plt;

    const char *shstrtab;
    const char *strtab;
    const char *dynstr;

    size_t symtab_num;
    size_t dynsym_num;
    size_t dynamic_num;
    size_t rela_dyn_num;
    size_t rela_plt_num;

    size_t shstrtab_len;
    size_t strtab_len;
    size_t dynstr_len;

    bool need_load_bias;  // PIE & shared object needs ASLR adjustment
    Elf_Addr vma_offset;  // .text page-aligned offset from the minimum load address
    Elf_Addr load_offset; // .text load segment vma - offset

    Elf_Addr tls_size;
    Elf_Addr tls_align;

    Elf_Addr plt_addr;
    Elf_Addr got_addr;
    size_t plt_size;
    size_t got_size;
};

/* target function record */
struct target_function {
    u64 addr;                        // target function address
    size_t count;                    // target function patch count
    struct list_head func_node;      // target function list node
};

struct target_entity {
    struct target_metadata meta;      // target file metadata
    struct hlist_node table_node;     // global target hash table node

    struct rw_semaphore action_rwsem; // target action rw semaphore

    struct list_head loaded_list;     // target loaded patches
    struct list_head actived_list;    // target actived patches
    struct list_head func_list;       // target registered functions

    struct mutex process_mutex;
    struct list_head process_list;    // all processes of the target
};

/*
 * Load a target entity
 * @param file_path: target file path
 * @return target entity
 */
struct target_entity *new_target_entity(const char *file_path);

/*
 * Free a target entity
 * @param target: target entity
 * @return void
 */
void free_target_entity(struct target_entity *target);

/*
 * Add a patch function to target entity
 * @param target: target entity
 * @param func: patch function
 * @param need_register: target offset needs register uprobe
 * @return result
 */
int target_add_function(struct target_entity *target, struct upatch_function *func, bool *need_register);

/*
 * Remove a patch function from target entity
 * @param target: target entity
 * @param func: patch function
 * @param need_unregister: target offset needs unregister uprobe
 * @return result
 */
void target_remove_function(struct target_entity *target, struct upatch_function *func, bool *need_unregister);

/*
 * Collect all exited process into a list
 * @param target: target entity
 * @param process_list: exited process list
 * @return void
 */
void target_gather_exited_processes(struct target_entity *target, struct list_head *process_list);

/*
 * Get or create a process entity from target entity
 * @param target: target entity
 * @return process entity
 */
struct process_entity *target_get_or_create_process(struct target_entity *target);

#endif // _UPATCH_MANAGE_TARGET_ENTITY_H
