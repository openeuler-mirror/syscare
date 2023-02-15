// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_PATCH_H
#define _UPATCH_PATCH_H

#include <linux/elf.h>

#include <asm/module.h>

#include "upatch-patch.h"
#include "upatch-manage.h"

/*
 * When patch works, it will no longer be controlled by the uprobe.
 * But we still need uprobe works in this situation to handler further threads.
 * Status definiations for threads:
 *      None: original status
 *     -----------------------
 *      Attached: register the uprobe handler
 *      Hacked: finish relocations
 *      Actived: jmp instructions works (release hack) --> wait for futher command + safety check
 *      Unactived: no jmp instructions (re-gain hack) --> register again ?
 *      Removed: unregister the uprobe handler -> actived threads will be restored?
 *
 *      used for the binary: apply / remove
 *      middle status: mmap
 *      used for the thread: actived / unactived
 *
 *      limit: self-modifications for funcs are forbidden.
 */

#define JMP_TABLE_MAX_ENTRY 100

/* jmp table, solve limit for the jmp instruction */
struct upatch_jmp_table_entry {
    unsigned long inst;
    unsigned long addr;
};

/* memory layout for module */
struct upatch_module_layout {
    /* The actual code + data. */
    void *kbase;
    void __user *base;
    /* Total size. */
    unsigned int size;
    /* The size of the executable code.  */
    unsigned int text_size;
    /* Size of RO section of the module (text+rodata) */
    unsigned int ro_size;
    /* Size of RO after init section, not use it now */
    unsigned int ro_after_init_size;
};

/* information to manage a patch module */
struct upatch_module {
    pid_t pid;
    struct list_head list;

    struct mutex module_status_lock;
    unsigned long load_bias;

    /* state changes happens asynchronously  */
    enum upatch_module_state real_state;
    struct inode *real_patch;

    /* memory layout for patch */
    struct upatch_module_layout core_layout;
    /* drop after init, we use it to store symtab and strtab */
    struct upatch_module_layout init_layout;

    /* address from module layout, consider record in memory */
    struct upatch_patch_func __user *upatch_funs;
    unsigned int num_upatch_funcs;
    char __user *strtab;
    Elf_Sym __user *syms;
    unsigned int num_syms;
};

struct uprobe_offset {
    loff_t offset;
    struct list_head list;
};

struct patch_entity {
    unsigned int ref;
    void *patch_buff;
    size_t patch_size;
};

struct upatch_entity {
    struct inode *binary;
    struct list_head list;

    /* protect any modification for this entity */
    struct mutex entity_status_lock;

    /* used to handle command */
    enum upatch_module_state set_status;
    struct inode *set_patch;

    /* sync with set_patch */
    struct patch_entity *patch_entity;

    struct list_head offset_list;
    struct list_head module_list;
};

struct upatch_load_info;
/* information needed to load running binary */
struct running_elf_info {
    unsigned long len;
    Elf_Ehdr *hdr;
    Elf_Shdr *sechdrs;
    char *secstrings, *strtab, *dynstrtab;
    /* minimal load address, used to calculate offset */
    unsigned long load_min;
    /* load bias, used to handle ASLR */
    unsigned long load_bias;
    struct {
        unsigned int sym, symstr;
        unsigned int dynsym, dynsymstr;
        unsigned int relaplt, reladyn;
	} index;
    struct upatch_load_info *load_info;
};

/* information for loading */
struct upatch_load_info {
    unsigned long len;
    Elf_Ehdr *hdr;
    Elf_Shdr *sechdrs;
    char *secstrings, *strtab;
    unsigned long symoffs, stroffs, core_typeoffs;
    unsigned long jmp_offs;
    unsigned int jmp_cur_entry, jmp_max_entry;
    struct {
		unsigned int sym, str;
	} index;
    struct upatch_module *mod;
    struct running_elf_info running_elf;
};

struct elf_build_id {
    struct {
        Elf64_Nhdr nhdr;
        char name[4];
    } head;
    uint8_t *id;
};

/* entity/module releated */
struct upatch_entity *upatch_entity_get(struct inode *);
struct upatch_module *upatch_module_get_or_create(struct upatch_entity *, pid_t);
void upatch_module_deallocate(struct upatch_module *);
void upatch_put_patch_entity(struct patch_entity *);
void upatch_get_patch_entity(struct patch_entity *);

/* management releated */
int upatch_attach(const char *, const char *);
int upatch_load(struct file *, struct inode *, struct patch_entity *,
    struct upatch_load_info *);

#endif /* _UPATCH_PATCH_H */
