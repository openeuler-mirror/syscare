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
#include <linux/limits.h>
#include <linux/err.h>

#include <linux/mutex.h>
#include <linux/spinlock.h>
#include <linux/hashtable.h>
#include <linux/kref.h>

#include <linux/elf.h>
#include <linux/module.h>

#define PATCH_HASH_BITS   4  // Single patch target would have less than 16 patches
#define UPROBE_HASH_BITS  7  // Single patch target would have less than 128 uprobes
#define PROCESS_HASH_BITS 4  // Single patch target would have less than 16 processes

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

struct file;
struct inode;

struct patch_entity;
struct upatch_function;

/* Target file */
struct target_file {
    char path_buff[PATH_MAX];

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

/* Target entity */
struct target_entity {
    struct target_file file;                       // target file

    struct hlist_node node;                        // hash table node
    bool is_deleting;                              // marker for deleting from hash table

    DECLARE_HASHTABLE(patches, PATCH_HASH_BITS);   // all loaded patches
    DECLARE_HASHTABLE(uprobes, UPROBE_HASH_BITS);  // all registered uprobes
    struct mutex patch_lock;

    struct list_head actived_patches;              // actived patch list
    spinlock_t active_lock;

    DECLARE_HASHTABLE(processes, PROCESS_HASH_BITS);  // all running processes
    spinlock_t process_lock;

    struct kref kref;
};

/**
 * @brief Load a new target file
 * @param file: Target file struct pointer
 * @return Newly allocated target entity with refcount=1, or NULL on failure
 *
 * Allocates and initializes a new taget entity structure with reference count 1.
 * The caller is responsible for calling put_target() when done.
 */
struct target_entity *load_target(struct file *file);

/**
 * @brief Release target resources when refcount reaches zero
 * @param kref: Reference counter
 *
 * Called automatically by kref_put().
 * Frees all target resources and disassociates from target.
 */
void release_target(struct kref *kref);

/**
 * @brief Acquire a reference to a target entity
 * @param target: Target entity pointer
 * @return Target entity with incremented refcount, or NULL if input is NULL
 *
 * Caller must balance with put_target().
 */
static inline struct target_entity *get_target(struct target_entity *target)
{
    if (unlikely(IS_ERR_OR_NULL(target))) {
        return NULL;
    }

    kref_get(&target->kref);
    return target;
}

/**
 * @brief Release a target entity reference
 * @param target: Target entity
 *
 * Decrements refcount and triggers release_target() when reaching zero.
 * Safe to call with NULL.
 */
static inline void put_target(struct target_entity *target)
{
    if (unlikely(IS_ERR_OR_NULL(target))) {
        return;
    }

    kref_put(&target->kref, release_target);
}

/**
 * @brief Load a patch to the target
 * @param target Target entity
 * @param patch Fully initialized patch entity
 * @return 0 on success, negative error code on failure
 */
int target_load_patch(struct target_entity *target, const char *filename);

/**
 * @brief Remove a patch from the target
 * @param target Target entity
 * @param inode Patch inode
 * @return 0 on success, negative error code on failure
 */
int target_remove_patch(struct target_entity *target, struct inode *inode);

/**
 * @brief Activate a patch on the target
 * @param target Target entity
 * @param inode Patch inode
 * @param uc Uprobe consumer
 * @return 0 on success, negative error code on failure
 */
int target_active_patch(struct target_entity *target, struct inode *inode, struct uprobe_consumer *uc);

/**
 * @brief Deactivate a patch on the target
 * @param target Target entity
 * @param inode Patch inode
 * @param uc Uprobe consumer
 * @return 0 on success, negative error code on failure
 */
int target_deactive_patch(struct target_entity *target, struct inode *inode, struct uprobe_consumer *uc);

/**
 * @brief Get patch status on the target
 * @param target Target entity
 * @param inode Patch inode
 * @return Patch entity
 */
enum upatch_status target_patch_status(struct target_entity *target, const struct inode *inode);

/**
 * @brief Get current actived patch on the target
 * @param target Target entity
 * @return Current actived patch entity or NULL if none exists
 *
 * The returned patch has its reference count incremented.
 * Caller must call put_patch() when done.
 */
struct patch_entity *target_get_actived_patch(struct target_entity *target);

/**
 * @brief Get or create process entity
 * @param target Target entity
 * @param task Process task_struct
 * @return Process entity with incremented refcount
 *
 * Caller must call put_process() when done.
 */
struct process_entity *target_get_process(struct target_entity *target, struct task_struct *task);

/**
 * @brief Cleanup exited processes of the target
 * @param target Target entity
 */
void target_cleanup_process(struct target_entity *target);

#endif // _UPATCH_MANAGE_TARGET_ENTITY_H
