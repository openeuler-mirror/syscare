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

#include "target_entity.h"

#include <linux/fs.h>

#include <linux/list.h>
#include <linux/hash.h>
#include <linux/hashtable.h>

#include "patch_entity.h"
#include "process_entity.h"
#include "util.h"

#define ELF_ADDR_MAX UINT_MAX

static const char *SHSTRTAB_NAME = ".shstrtab";
static const char *STRTAB_NAME   = ".strtab";
static const char *DYNSTR_NAME   = ".dynstr";
static const char *DYN_RELA_NAME = ".rela.dyn";
static const char *PLT_RELA_NAME = ".rela.plt";
static const char *PLT_NAME      = ".plt";
static const char *GOT_NAME      = ".got";

struct uprobe_record {
    struct hlist_node node;

    loff_t offset;
    long count;
};

/* --- Forward declarations --- */

static void destroy_target_file(struct target_file *target_file);

/* --- Target life-cycle management --- */

static int resolve_file_sections(struct target_file *target_file, struct file *file)
{
    Elf_Shdr *shdrs = target_file->shdrs;
    Elf_Half shdr_num = target_file->ehdr->e_shnum;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab;
    size_t shstrtab_len;

    const char *sec_name;
    void *sec_data;

    shdr = &shdrs[target_file->ehdr->e_shstrndx];
    shstrtab = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(shstrtab)) {
        log_err("failed to read section '%s'\n", SHSTRTAB_NAME);
        return PTR_ERR(shstrtab);
    }
    shstrtab_len = shdr->sh_size;

    target_file->shstrtab = shstrtab;
    target_file->shstrtab_len = shstrtab_len;

    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        sec_name = get_string_at(shstrtab, shstrtab_len, shdr->sh_name);
        if (sec_name == NULL) {
            log_err("invalid section name, index=%u\n", i);
            return -ENOEXEC;
        }

        switch (shdr->sh_type) {
            case SHT_SYMTAB:
                sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                target_file->symtab = sec_data;
                target_file->symtab_num = shdr->sh_size / sizeof(Elf_Sym);
                break;

            case SHT_STRTAB:
                if (strcmp(sec_name, STRTAB_NAME) == 0) {
                    sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                    target_file->strtab = sec_data;
                    target_file->strtab_len = shdr->sh_size;
                } else if (strcmp(sec_name, DYNSTR_NAME) == 0) {
                    sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                    target_file->dynstr = sec_data;
                    target_file->dynstr_len = shdr->sh_size;
                }
                break;

            case SHT_RELA:
                if (strcmp(sec_name, DYN_RELA_NAME) == 0) {
                    sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                    target_file->rela_dyn = sec_data;
                    target_file->rela_dyn_num = shdr->sh_size / sizeof(Elf_Rela);
                } else if (strcmp(sec_name, PLT_RELA_NAME) == 0) {
                    sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                    target_file->rela_plt = sec_data;
                    target_file->rela_plt_num = shdr->sh_size / sizeof(Elf_Rela);
                }
                break;

            case SHT_DYNAMIC:
                sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                target_file->dynamic = sec_data;
                target_file->dynamic_num = shdr->sh_size / sizeof(Elf_Dyn);
                break;

            case SHT_DYNSYM:
                sec_data = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                target_file->dynsym = sec_data;
                target_file->dynsym_num = shdr->sh_size / sizeof(Elf_Sym);
                break;

            default:
                // do nothing
                break;
        }

        if (IS_ERR(sec_data)) {
            log_err("failed to read section '%s'\n", sec_name);
            return PTR_ERR(sec_data);
        }
    }

    return 0;
}

static int resolve_file_address(struct target_file *target_file)
{
    Elf_Addr min_load_addr = ELF_ADDR_MAX;
    bool found_text_segment = false;

    Elf_Ehdr *ehdr = target_file->ehdr;
    Elf_Phdr *phdrs = target_file->phdrs;
    Elf_Half phdr_num = target_file->ehdr->e_phnum;
    Elf_Phdr *phdr;

    Elf_Shdr *shdrs = target_file->shdrs;
    Elf_Half shdr_num = target_file->ehdr->e_shnum;
    Elf_Shdr *shdr;

    const char *shstrtab = target_file->shstrtab;
    const char *sec_name;

    Elf_Half i;

    /*
     * Check if the ELF file is position-independent (ET_DYN type).
     * This includes:
     *   - Shared libraries (.so)
     *   - Position-Independent Executables (PIE)
     * Such files require load bias adjustment (ASLR support).
     * Non-PIE executables (ET_EXEC) load at fixed addresses and don't need bias.
     */
    if (ehdr->e_type == ET_DYN) {
        target_file->need_load_bias = true;
    } else {
        target_file->need_load_bias = false;
    }

    /* find minimum load virtual address */
    for (i = 0; i < phdr_num; i++) {
        phdr = &phdrs[i];
        if (phdr->p_type == PT_LOAD) {
            min_load_addr = min(min_load_addr, phdr->p_vaddr);
        }
    }
    if (min_load_addr == ELF_ADDR_MAX) {
        log_err("cannot find any PT_LOAD segment\n");
        return -ENOEXEC;
    }

    /* parse program headers */
    for (i = 0; i < phdr_num; i++) {
        phdr = &phdrs[i];

        switch (phdr->p_type) {
            case PT_LOAD: {
                if (phdr->p_flags & PF_X) {
                    if (found_text_segment) {
                        log_err("found multiple executable PT_LOAD segments\n");
                        return -ENOEXEC;
                    }
                    target_file->vma_offset = (phdr->p_vaddr - min_load_addr) & PAGE_MASK;
                    target_file->load_offset = phdr->p_vaddr - min_load_addr - phdr->p_offset;
                    found_text_segment = true;
                }
                break;
            }

            case PT_TLS:
                target_file->tls_size = phdr->p_memsz;
                target_file->tls_align = phdr->p_align;
                break;

            default:
                break;
        }
    }

    if (!found_text_segment) {
        log_err("no executable PT_LOAD segment\n");
        return -ENOEXEC;
    }

    /* parse section headers */
    for (i = 0; i < shdr_num; i++) {
        if (target_file->plt_addr && target_file->got_addr) {
            break;
        }

        shdr = &shdrs[i];
        if (shdr->sh_type != SHT_PROGBITS) {
            continue;
        }

        sec_name = shstrtab + shdr->sh_name;
        if ((shdr->sh_flags & (SHF_ALLOC|SHF_EXECINSTR)) == (SHF_ALLOC|SHF_EXECINSTR)) {
            if (!target_file->plt_addr && strcmp(sec_name, PLT_NAME) == 0) {
                target_file->plt_addr = shdr->sh_addr;
                target_file->plt_size = shdr->sh_size;
            }
        }
        if ((shdr->sh_flags & (SHF_ALLOC|SHF_WRITE)) == (SHF_ALLOC|SHF_WRITE)) {
            if (!target_file->got_addr && strcmp(sec_name, GOT_NAME) == 0) {
                target_file->got_addr = shdr->sh_addr;
                target_file->got_size = shdr->sh_size;
            }
        }
    }

    log_debug("vma_offset: 0x%llx, load_offset: 0x%llx, plt_addr: 0x%llx, got_addr: 0x%llx\n",
        target_file->vma_offset, target_file->load_offset, target_file->plt_addr, target_file->got_addr);
    return 0;
}

static int resolve_target_file(struct target_file *target_file, struct file *file)
{
    int ret;

    rcu_read_lock();
    target_file->path = file_path(file, target_file->path_buff, PATH_MAX);
    rcu_read_unlock();

    if (unlikely(IS_ERR(target_file->path))) {
        ret = PTR_ERR(target_file->path);
        log_err("faild to get file path\n");
        goto out;
    }

    target_file->inode = igrab(file_inode(file));
    if (unlikely(!target_file->inode)) {
        log_err("failed to get file inode\n");
        ret = -ENOENT;
        goto out;
    }

    target_file->size = i_size_read(target_file->inode);

    target_file->ehdr = vmalloc_read(file, 0, sizeof(Elf_Ehdr));
    if (unlikely(IS_ERR(target_file->ehdr))) {
        log_err("failed to read elf header\n");
        ret = PTR_ERR(target_file->ehdr);
        goto out;
    }

    if (unlikely(!is_valid_target(target_file->ehdr, target_file->size))) {
        log_err("invalid file format\n");
        ret = -ENOEXEC;
        goto out;
    }

    target_file->phdrs = vmalloc_read(file, target_file->ehdr->e_phoff,
        target_file->ehdr->e_phentsize * target_file->ehdr->e_phnum);
    if (unlikely(IS_ERR(target_file->phdrs))) {
        log_err("failed to read program header\n");
        ret = PTR_ERR(target_file->phdrs);
        goto out;
    }

    target_file->shdrs = vmalloc_read(file, target_file->ehdr->e_shoff,
        target_file->ehdr->e_shentsize * target_file->ehdr->e_shnum);
    if (unlikely(IS_ERR(target_file->shdrs))) {
        log_err("failed to read section header\n");
        ret = PTR_ERR(target_file->shdrs);
        goto out;
    }

    ret = resolve_file_sections(target_file, file);
    if (unlikely(ret)) {
        log_err("failed to resolve target sections\n");
        goto out;
    }

    ret = resolve_file_address(target_file);
    if (unlikely(ret)) {
        log_err("failed to resolve target address\n");
        goto out;
    }

out:
    if (ret) {
        destroy_target_file(target_file);
    }
    return ret;
}

static void destroy_target_file(struct target_file *target_file)
{
    iput(target_file->inode);

    target_file->path = NULL;
    target_file->inode = NULL;
    target_file->size = 0;

    VFREE_CLEAR(target_file->ehdr);
    VFREE_CLEAR(target_file->phdrs);
    VFREE_CLEAR(target_file->shdrs);

    VFREE_CLEAR(target_file->symtab);
    VFREE_CLEAR(target_file->dynsym);
    VFREE_CLEAR(target_file->dynamic);
    VFREE_CLEAR(target_file->rela_dyn);
    VFREE_CLEAR(target_file->rela_plt);

    VFREE_CLEAR(target_file->shstrtab);
    VFREE_CLEAR(target_file->strtab);
    VFREE_CLEAR(target_file->dynstr);

    target_file->symtab_num = 0;
    target_file->dynamic_num = 0;
    target_file->dynsym_num = 0;
    target_file->rela_dyn_num = 0;
    target_file->rela_plt_num = 0;

    target_file->shstrtab_len = 0;
    target_file->strtab_len = 0;
    target_file->dynstr_len = 0;

    target_file->need_load_bias = false;
    target_file->vma_offset = 0;
    target_file->load_offset = 0;

    target_file->tls_size = 0;
    target_file->tls_align = 0;

    target_file->plt_addr = 0;
    target_file->got_addr = 0;
    target_file->plt_size = 0;
    target_file->got_size = 0;
}

static int resolve_target(struct target_entity *target, struct file *file)
{
    int ret;

    ret = resolve_target_file(&target->file, file);
    if (unlikely(ret)) {
        return ret;
    }

    INIT_HLIST_NODE(&target->node);
    target->is_deleting = false;

    hash_init(target->patches);
    hash_init(target->uprobes);
    mutex_init(&target->patch_lock);

    INIT_LIST_HEAD(&target->actived_patches);
    spin_lock_init(&target->active_lock);

    hash_init(target->processes);
    spin_lock_init(&target->process_lock);

    kref_init(&target->kref);

    return 0;
}

static void destroy_target(struct target_entity *target)
{
    struct process_entity *process;
    struct hlist_node *tmp;
    int bkt;

    WARN_ON(!hlist_unhashed(&target->node));
    target->is_deleting = false;

    destroy_target_file(&target->file);

    WARN_ON(mutex_is_locked(&target->patch_lock));
    WARN_ON(!hash_empty(target->patches));
    WARN_ON(!hash_empty(target->uprobes));
    hash_init(target->patches);
    hash_init(target->uprobes);

    WARN_ON(spin_is_locked(&target->active_lock));
    WARN_ON(!list_empty(&target->actived_patches));

    WARN_ON(spin_is_locked(&target->process_lock));
    hash_for_each_safe(target->processes, bkt, tmp, process, node) {
        hash_del(&process->node);
        put_process(process);
    }
    hash_init(target->processes);
}

/* --- Patch management --- */

static inline struct patch_entity *find_patch_unlocked(const struct target_entity *target, const struct inode *inode)
{
    struct patch_entity *patch;

    hash_for_each_possible(target->patches, patch, node, hash_inode(inode, PATCH_HASH_BITS)) {
        if (inode_equal(patch->file.inode, inode)) {
            return patch;
        }
    }

    return NULL;
}

static inline struct uprobe_record *find_function_unlocked(const struct target_entity *target, u64 offset)
{
    struct uprobe_record *uprobe;

    hash_for_each_possible(target->uprobes, uprobe, node, hash_64(offset, UPROBE_HASH_BITS)) {
        if (uprobe->offset == offset) {
            return uprobe;
        }
    }

    return NULL;
}

static int register_uprobe_unlocked(struct target_entity *target, const struct upatch_function *func,
    struct uprobe_consumer *uc)
{
    struct uprobe_record *uprobe;
    int ret;

    uprobe = find_function_unlocked(target, func->old_addr);
    if (uprobe) {
        uprobe->count += 1;
        return 0;
    }

    uprobe = kmalloc(sizeof(*uprobe), GFP_KERNEL);
    if (unlikely(!uprobe)) {
        return -ENOMEM;
    }

    INIT_HLIST_NODE(&uprobe->node);
    uprobe->offset = func->old_addr;
    uprobe->count = 1;

    ret = uprobe_register(target->file.inode, uprobe->offset, uc);
    if (unlikely(ret)) {
        log_err("failed to register uprobe on '%s', offset=0x%llx\n", target->file.path, uprobe->offset);
        kfree(uprobe);
        return ret;
    }

    hash_add(target->uprobes, &uprobe->node, hash_64(func->old_addr, UPROBE_HASH_BITS));

    return ret;
}

static void unregister_uprobe_unlocked(struct target_entity *target, const struct upatch_function *func,
    struct uprobe_consumer *uc)
{
    struct uprobe_record *uprobe;

    uprobe = find_function_unlocked(target, func->old_addr);
    if (!uprobe) {
        return;
    }

    uprobe->count -= 1;
    if (uprobe->count) {
        return; // still has reference
    }

    hash_del(&uprobe->node);
    uprobe_unregister(target->file.inode, uprobe->offset, uc);
    kfree(uprobe);
}

static void do_unregister_patch_functions_unlocked(struct target_entity *target, const struct patch_entity *patch,
    struct uprobe_consumer *uc, size_t count)
{
    const struct upatch_function *funcs = patch->file.funcs;
    const char *strings = patch->file.strings;

    const struct upatch_function *func;
    const char *name;
    size_t i;

    if (count > patch->file.func_num) {
        log_err("function count %zu exceeds %zu\n", count, patch->file.func_num);
        return;
    }

    log_debug("%s: unregister patch %s functions\n", target->file.path, patch->file.path);
    for (i = 0; i < count; i++) {
        func = &funcs[i];
        name = strings + func->name_off;

        log_debug("- function: offset=0x%08llx, size=0x%04llx, name='%s'\n", func->old_addr, func->old_size, name);
        unregister_uprobe_unlocked(target, func, uc);
    }
}

static void unregister_patch_functions_unlocked(struct target_entity *target, const struct patch_entity *patch,
    struct uprobe_consumer *uc)
{
    do_unregister_patch_functions_unlocked(target, patch, uc, patch->file.func_num);
}

static int register_patch_functions_unlocked(struct target_entity *target, const struct patch_entity *patch,
    struct uprobe_consumer *uc)
{
    const struct upatch_function *funcs = patch->file.funcs;
    const char *strings = patch->file.strings;

    const struct upatch_function *func;
    const char *name;
    size_t i;

    int ret = 0;

    log_debug("%s: register patch %s functions\n", target->file.path, patch->file.path);
    for (i = 0; i < patch->file.func_num; i++) {
        func = &funcs[i];
        name = strings + func->name_off;

        log_debug("+ function: offset=0x%08llx, size=0x%04llx, name='%s'\n", func->old_addr, func->old_size, name);
        ret = register_uprobe_unlocked(target, func, uc);
        if (ret) {
            log_err("failed to register function '%s'\n", name);
            do_unregister_patch_functions_unlocked(target, patch, uc, i); // we need rollback all changes
            break;
        }
    }

    return ret;
}

/* --- Process management --- */

static inline struct process_entity *find_process_unlocked(const struct target_entity *target, pid_t pid)
{
    struct process_entity *process;

    hash_for_each_possible(target->processes, process, node, hash_32(pid, PROCESS_HASH_BITS)) {
        if (process->tgid == pid) {
            return process;
        }
    }

    return NULL;
}

static int check_patch_removable(struct target_entity *target, struct patch_entity *patch)
{
    struct process_entity *process;
    int bkt;

    int ret = 0;

    spin_lock(&target->process_lock);

    hash_for_each(target->processes, bkt, process, node) {
        ret = process_check_patch_on_stack(process, patch);
        if (ret) {
            break;
        }
    }

    spin_unlock(&target->process_lock);

    return ret;
}

static inline void remove_patch_on_all_process(struct target_entity *target, struct patch_entity *patch)
{
    struct process_entity *process;
    int bkt;

    hash_for_each(target->processes, bkt, process, node) {
        spin_lock(&process->thread_lock);
        process_remove_patch(process, patch);
        spin_unlock(&process->thread_lock);
    }
}

/* --- Public interface --- */

struct target_entity *load_target(struct file *file)
{
    struct target_entity *target = NULL;
    int ret;

    if (unlikely(!file)) {
        return ERR_PTR(-EINVAL);
    }

    target = kzalloc(sizeof(struct target_entity), GFP_KERNEL);
    if (unlikely(!target)) {
        return ERR_PTR(-ENOMEM);
    }

    ret = resolve_target(target, file);
    if (unlikely(ret)) {
        kfree(target);
        return ERR_PTR(ret);
    }

    log_debug("new target %s\n", target->file.path);
    return target;
}

void release_target(struct kref *kref)
{
    struct target_entity *target;

    if (unlikely(!kref)) {
        return;
    }

    target = container_of(kref, struct target_entity, kref);
    log_debug("free target %s\n", target->file.path);

    destroy_target(target);
    kfree(target);
}

int target_load_patch(struct target_entity *target, const char *filename)
{
    struct file *file;
    struct patch_entity *existing;
    struct patch_entity *patch;

    struct patch_entity *to_be_freed = NULL;
    int ret = 0;

    if (unlikely(!target || !filename)) {
        return -EINVAL;
    }

    /* --- Fast path: quick check if the patch is already exists --- */

    /* Step 1: Open the file */
    file = filp_open(filename, O_RDONLY, 0);
    if (unlikely(IS_ERR(file))) {
        return PTR_ERR(file);
    }

    mutex_lock(&target->patch_lock);

    /* Step 2: Check if the patch is already exists */
    existing = find_patch_unlocked(target, file_inode(file));
    if (unlikely(existing)) {
        ret = -EEXIST;
        goto unlock_out;
    }

    mutex_unlock(&target->patch_lock);

    /* --- Slow path: load patch, check existence, insert patch table --- */

    /* Step 3: Load patch from file */
    patch = load_patch(file);
    if (unlikely(IS_ERR(patch))) {
        ret = PTR_ERR(patch);
        goto release_out;
    }
    patch->status = UPATCH_STATUS_DEACTIVED;

    mutex_lock(&target->patch_lock);

    /* Step 4: Re-check if the patch already exists (to handle race) */
    existing = find_patch_unlocked(target, patch->file.inode);
    if (unlikely(existing)) {
        ret = -EEXIST;
        to_be_freed = patch;
        goto unlock_out;
    }

    /* Step 5: Insert the patch into patch table */
    hash_add(target->patches, &patch->node, hash_inode(patch->file.inode, PATCH_HASH_BITS));

unlock_out:
    mutex_unlock(&target->patch_lock);

release_out:
    filp_close(file, NULL);
    put_patch(to_be_freed);

    return ret;
}

int target_remove_patch(struct target_entity *target, struct inode *inode)
{
    struct patch_entity *patch;

    struct patch_entity *to_be_freed = NULL;
    int ret = 0;

    if (unlikely(!target || !inode)) {
        return -EINVAL;
    }

    mutex_lock(&target->patch_lock);

    /* Step 1: Find patch from target patch table */
    patch = find_patch_unlocked(target, inode);
    if (unlikely(!patch)) {
        ret = -ENOENT; // patch does not exist
        goto unlock_out;
    }

    /* Step 2: Verify the patch status is what we expected */
    if (patch->status != UPATCH_STATUS_DEACTIVED) {
        ret = -EPERM;
        goto unlock_out;
    }

    /* Step 3: Check if the patch is removable */
    ret = check_patch_removable(target, patch);
    if (unlikely(ret)) {
        goto unlock_out;
    }

    /* Step 4: Remove the patch from all processes */
    remove_patch_on_all_process(target, patch);

    /* Step 5: Remove patch from target patch table & mark removable */
    hash_del(&patch->node);
    to_be_freed = patch;

    /* Step 6: Update the patch status */
    patch->status = UPATCH_STATUS_NOT_APPLIED;

    /*
     * We still have the ownership of the patch, since it was removed from target patch table and we didn't release it.
     * Thus, we don't need increase it's reference.
     */
unlock_out:
    mutex_unlock(&target->patch_lock);

    put_patch(to_be_freed);
    return ret;
}

int target_active_patch(struct target_entity *target, struct inode *inode, struct uprobe_consumer *uc)
{
    struct patch_entity *patch;

    int ret = 0;

    if (unlikely(!target || !inode || !uc)) {
        return -EINVAL;
    }

    mutex_lock(&target->patch_lock);

    /* Step 1: Find patch from target patch table */
    patch = find_patch_unlocked(target, inode);
    if (unlikely(!patch)) {
        ret = -ENOENT; // patch does not exist
        goto unlock_out;
    }

    /* Step 2: Verify the patch status is what we expected */
    if (unlikely(patch->status != UPATCH_STATUS_DEACTIVED)) {
        ret = -EPERM;
        goto unlock_out;
    }

    /* Step 3: Register the patch functions */
    ret = register_patch_functions_unlocked(target, patch, uc);
    if (unlikely(ret)) {
        goto unlock_out;
    }

    /* Step 4: Insert the patch into target actived patch list */
    spin_lock(&target->active_lock);
    list_add(&patch->actived_node, &target->actived_patches);
    get_patch(patch);
    spin_unlock(&target->active_lock);

    /* Step 5: Update the patch status */
    patch->status = UPATCH_STATUS_ACTIVED;

unlock_out:
    mutex_unlock(&target->patch_lock);

    return ret;
}

int target_deactive_patch(struct target_entity *target, struct inode *inode, struct uprobe_consumer *uc)
{
    struct patch_entity *patch;

    struct patch_entity *to_be_freed = NULL;
    int ret = 0;

    if (unlikely(!target || !inode || !uc)) {
        return -EINVAL;
    }

    mutex_lock(&target->patch_lock);

    /* Step 1: Find patch from target patch table */
    patch = find_patch_unlocked(target, inode);
    if (unlikely(!patch)) {
        ret = -ENOENT; // patch does not exist
        goto unlock_out;
    }

    /* Step 2: Verify the patch status is what we expected */
    if (unlikely(patch->status != UPATCH_STATUS_ACTIVED)) {
        ret = -EPERM;
        goto unlock_out;
    }

    /* Step 3: Register the patch functions */
    unregister_patch_functions_unlocked(target, patch, uc);

    /* Step 4: Remove the patch from target actived patch list */
    spin_lock(&target->active_lock);
    list_del_init(&patch->actived_node);
    to_be_freed = patch;
    spin_unlock(&target->active_lock);

    /* Step 5: Update patch status */
    patch->status = UPATCH_STATUS_DEACTIVED;

unlock_out:
    mutex_unlock(&target->patch_lock);

    put_patch(to_be_freed);
    return ret;
}

enum upatch_status target_patch_status(struct target_entity *target, const struct inode *inode)
{
    enum upatch_status ret = UPATCH_STATUS_NOT_APPLIED;
    struct patch_entity *patch;

    if (unlikely(!target || !inode)) {
        return ret;
    }

    mutex_lock(&target->patch_lock);

    /* Step 1: Find patch from target patch table */
    patch = find_patch_unlocked(target, inode);
    if (unlikely(!patch)) {
        goto unlock_out; // patch does not exist
    }

    /* Step 2: Get patch status */
    ret = patch->status;

unlock_out:
    mutex_unlock(&target->patch_lock);

    return ret;
}

struct patch_entity *target_get_actived_patch(struct target_entity *target)
{
    struct patch_entity *patch;

    spin_lock(&target->active_lock);
    patch = get_patch(list_first_entry_or_null(&target->actived_patches, struct patch_entity, actived_node));
    spin_unlock(&target->active_lock);

    return patch;
}

struct process_entity *target_get_process(struct target_entity *target, struct task_struct *task)
{
    struct process_entity *process;
    pid_t pid = task_tgid_nr(task);

    if (unlikely(!target)) {
        return ERR_PTR(-EINVAL);
    }

    spin_lock(&target->process_lock);

    process = find_process_unlocked(target, pid);
    if (!process) {
        log_debug("create process %d for '%s'\n", pid, target->file.path);

        process = new_process(task);
        if (IS_ERR(process)) {
            log_err("failed to create target process, ret=%d\n", (int)PTR_ERR(process));
            goto unlock_out;
        }

        hash_add(target->processes, &process->node, hash_32(pid, PROCESS_HASH_BITS));
    }

    get_process(process);

unlock_out:
    spin_unlock(&target->process_lock);

    return process;
}

void target_cleanup_process(struct target_entity *target)
{
    struct process_entity *process;
    struct process_entity *n;
    struct hlist_node *tmp;
    int bkt;

    LIST_HEAD(pending_list);

    if (unlikely(!target)) {
        return;
    }

    spin_lock(&target->process_lock);

    hash_for_each_safe(target->processes, bkt, tmp, process, node) {
        if (!process_is_alive(process)) {
            hash_del(&process->node);
            list_add(&process->pending_node, &pending_list);
        }
    }

    spin_unlock(&target->process_lock);

    list_for_each_entry_safe(process, n, &pending_list, pending_node) {
        list_del_init(&process->pending_node);
        put_process(process);
    }
}
