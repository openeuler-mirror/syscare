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
struct upatch_function;

/* target elf metadata */
struct target_metadata {
    const char *file_name;
    loff_t file_size;

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

struct target_entity {
    const char *path;               // patch file path
    struct inode *inode;            // target file inode

    struct target_metadata meta;    // target file elf data

    /*
     * there is only one thread to call active / deactive
     * we don't need a lock
     */
    struct list_head offset_node;   // list of file offset of active patch function for struct patched_offset
    struct hlist_node node;         // all target store in hash table

    /*
     * all patches related to this target, including active and deactive patches
     * don't need lock. only load_patch, remove_patch, rmmod upatch_manage will read/write this list
     * uprobe_handle will not use this list, and we limit there is only one thread to manage patch
     */
    struct list_head all_patch_list;

    /*
     * active patch list need lock
     * uprobe handle will read it, active method will write it
     */
    struct rw_semaphore patch_lock;
    struct list_head actived_patch_list;

    /*
     * target ELF may run in different process, such as a dynamic object
     * every process will have a actived patch
     */
    struct mutex process_lock;      // uprobe handle will call free_process, so we need lock
    struct list_head process_head;
};

/* Patched address */
struct patched_offset {
    loff_t offset;                  // offset of the patched func addr
    struct list_head funcs_head;    // patched function list head
    struct list_head list;          // address list node
};

/* Patched function record */
struct patched_func_node {
    struct upatch_function *func;   // patched function
    struct list_head list;
};

/*
 * Find a target entity
 * @param ino: target file inode number
 * @return target entity
 */
struct target_entity *get_target_entity(const char* target_path);

struct target_entity *get_target_entity_by_inode(struct inode *inode);

/*
 * Load a target entity
 * @param file_path: target file path
 * @return target entity
 */
struct target_entity *new_target_entity(const char *file_path);

/*
 * Remove a target entity
 * @param target: target entity
 * @return void
 */
void free_target_entity(struct target_entity *target);

/*
 * Check if a target has related patches. DEACTIVE/ACTIVE patches are all counted
 * @param target: target entity
 * @param offset: target offset
 * @return result
 */
bool is_target_has_patch(const struct target_entity *target);

void __exit report_target_table_populated(void);

#endif // _UPATCH_MANAGE_TARGET_ENTITY_H
