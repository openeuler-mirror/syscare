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

static int resolve_target_sections(struct target_metadata *meta, struct file *target)
{
    Elf_Shdr *shdrs = meta->shdrs;
    Elf_Half shdr_num = meta->ehdr->e_shnum;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab;
    size_t shstrtab_len;

    const char *sec_name;
    void *sec_data;

    shdr = &shdrs[meta->ehdr->e_shstrndx];
    shstrtab = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(shstrtab)) {
        log_err("failed to read section '%s'\n", SHSTRTAB_NAME);
        return PTR_ERR(shstrtab);
    }
    shstrtab_len = shdr->sh_size;

    meta->shstrtab = shstrtab;
    meta->shstrtab_len = shstrtab_len;

    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        sec_name = get_string_at(shstrtab, shstrtab_len, shdr->sh_name);
        if (sec_name == NULL) {
            log_err("invalid section name, index=%u\n", i);
            return -ENOEXEC;
        }

        switch (shdr->sh_type) {
            case SHT_SYMTAB:
                sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->symtab = sec_data;
                meta->symtab_num = shdr->sh_size / sizeof(Elf_Sym);
                break;

            case SHT_STRTAB:
                if (strcmp(sec_name, STRTAB_NAME) == 0) {
                    sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->strtab = sec_data;
                    meta->strtab_len = shdr->sh_size;
                } else if (strcmp(sec_name, DYNSTR_NAME) == 0) {
                    sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->dynstr = sec_data;
                    meta->dynstr_len = shdr->sh_size;
                }
                break;

            case SHT_RELA:
                if (strcmp(sec_name, DYN_RELA_NAME) == 0) {
                    sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_dyn = sec_data;
                    meta->rela_dyn_num = shdr->sh_size / sizeof(Elf_Rela);
                } else if (strcmp(sec_name, PLT_RELA_NAME) == 0) {
                    sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_plt = sec_data;
                    meta->rela_plt_num = shdr->sh_size / sizeof(Elf_Rela);
                }
                break;

            case SHT_DYNAMIC:
                sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->dynamic = sec_data;
                meta->dynamic_num = shdr->sh_size / sizeof(Elf_Dyn);
                break;

            case SHT_DYNSYM:
                sec_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->dynsym = sec_data;
                meta->dynsym_num = shdr->sh_size / sizeof(Elf_Sym);
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

static int resolve_target_address(struct target_metadata *meta)
{
    Elf_Addr min_load_addr = ELF_ADDR_MAX;
    bool found_text_segment = false;

    Elf_Ehdr *ehdr = meta->ehdr;
    Elf_Phdr *phdrs = meta->phdrs;
    Elf_Half phdr_num = meta->ehdr->e_phnum;
    Elf_Phdr *phdr;

    Elf_Shdr *shdrs = meta->shdrs;
    Elf_Half shdr_num = meta->ehdr->e_shnum;
    Elf_Shdr *shdr;

    const char *shstrtab = meta->shstrtab;
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
        meta->need_load_bias = true;
    } else {
        meta->need_load_bias = false;
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
                    meta->vma_offset = (phdr->p_vaddr - min_load_addr) & PAGE_MASK;
                    meta->load_offset = phdr->p_vaddr - min_load_addr - phdr->p_offset;
                    found_text_segment = true;
                }
                break;
            }

            case PT_TLS:
                meta->tls_size = phdr->p_memsz;
                meta->tls_align = phdr->p_align;
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
    for (Elf_Half i = 0; i < shdr_num; i++) {
        if (meta->plt_addr && meta->got_addr) {
            break;
        }

        shdr = &shdrs[i];
        if (shdr->sh_type != SHT_PROGBITS) {
            continue;
        }

        sec_name = shstrtab + shdr->sh_name;
        if ((shdr->sh_flags & (SHF_ALLOC|SHF_EXECINSTR)) == (SHF_ALLOC|SHF_EXECINSTR)) {
            if (!meta->plt_addr && strcmp(sec_name, PLT_NAME) == 0) {
                meta->plt_addr = shdr->sh_addr;
                meta->plt_size = shdr->sh_size;
            }
        }
        if ((shdr->sh_flags & (SHF_ALLOC|SHF_WRITE)) == (SHF_ALLOC|SHF_WRITE)) {
            if (!meta->got_addr && strcmp(sec_name, GOT_NAME) == 0) {
                meta->got_addr = shdr->sh_addr;
                meta->got_size = shdr->sh_size;
            }
        }
    }

    log_debug("vma_offset: 0x%llx, load_offset: 0x%llx, plt_addr: 0x%llx, got_addr: 0x%llx\n",
        meta->vma_offset, meta->load_offset, meta->plt_addr, meta->got_addr);
    return 0;
}

static void destroy_target_metadata(struct target_metadata *meta)
{
    KFREE_CLEAR(meta->path);
    iput(meta->inode);
    meta->inode = NULL;

    meta->size = 0;

    VFREE_CLEAR(meta->ehdr);
    VFREE_CLEAR(meta->phdrs);
    VFREE_CLEAR(meta->shdrs);

    VFREE_CLEAR(meta->symtab);
    VFREE_CLEAR(meta->dynsym);
    VFREE_CLEAR(meta->dynamic);
    VFREE_CLEAR(meta->rela_dyn);
    VFREE_CLEAR(meta->rela_plt);

    VFREE_CLEAR(meta->shstrtab);
    VFREE_CLEAR(meta->strtab);
    VFREE_CLEAR(meta->dynstr);

    meta->symtab_num = 0;
    meta->dynamic_num = 0;
    meta->dynsym_num = 0;
    meta->rela_dyn_num = 0;
    meta->rela_plt_num = 0;

    meta->shstrtab_len = 0;
    meta->strtab_len = 0;
    meta->dynstr_len = 0;

    meta->need_load_bias = false;
    meta->vma_offset = 0;
    meta->load_offset = 0;

    meta->tls_size = 0;
    meta->tls_align = 0;

    meta->plt_addr = 0;
    meta->got_addr = 0;
    meta->plt_size = 0;
    meta->got_size = 0;
}

static int resolve_target_metadata(struct target_metadata *meta, const char *file_path)
{
    struct file *file;
    int ret;

    file = filp_open(file_path, O_RDONLY, 0);
    if (IS_ERR(file)) {
        log_err("failed to open '%s'\n", file_path);
        ret = PTR_ERR(file);
        file = NULL;
        goto out;
    }

    meta->path = kstrdup(file_path, GFP_KERNEL);
    if (!meta->path) {
        log_err("failed to alloc file path\n");
        ret = -ENOMEM;
        goto out;
    }

    meta->inode = igrab(file_inode(file));
    if (!meta->inode) {
        log_err("file '%s' inode is invalid\n", meta->path);
        ret = -ENOENT;
        goto out;
    }

    meta->size = i_size_read(meta->inode);

    meta->ehdr = vmalloc_read(file, 0, sizeof(Elf_Ehdr));
    if (IS_ERR(meta->ehdr)) {
        log_err("failed to read elf header\n");
        ret = PTR_ERR(meta->ehdr);
        goto out;
    }

    if (!is_valid_target(meta->ehdr, meta->size)) {
        log_err("invalid file format\n");
        ret = -ENOEXEC;
        goto out;
    }

    meta->phdrs = vmalloc_read(file, meta->ehdr->e_phoff, meta->ehdr->e_phentsize * meta->ehdr->e_phnum);
    if (IS_ERR(meta->phdrs)) {
        log_err("failed to read program header\n");
        ret = PTR_ERR(meta->phdrs);
        goto out;
    }

    meta->shdrs = vmalloc_read(file, meta->ehdr->e_shoff, meta->ehdr->e_shentsize * meta->ehdr->e_shnum);
    if (IS_ERR(meta->shdrs)) {
        log_err("failed to read section header\n");
        ret = PTR_ERR(meta->shdrs);
        goto out;
    }

    ret = resolve_target_sections(meta, file);
    if (ret) {
        log_err("failed to resolve target sections\n");
        goto out;
    }

    ret = resolve_target_address(meta);
    if (ret) {
        log_err("failed to resolve target address\n");
        goto out;
    }

out:
    if (file) {
        filp_close(file, NULL);
    }
    if (ret) {
        destroy_target_metadata(meta);
    }
    return ret;
}

static int resolve_target_entity(struct target_entity *target, const char *file_path)
{
    int ret;

    ret = resolve_target_metadata(&target->meta, file_path);
    if (ret) {
        return ret;
    }

    INIT_HLIST_NODE(&target->table_node);

    init_rwsem(&target->action_rwsem);
    INIT_LIST_HEAD(&target->loaded_list);
    INIT_LIST_HEAD(&target->actived_list);
    INIT_LIST_HEAD(&target->func_list);

    mutex_init(&target->process_mutex);
    INIT_LIST_HEAD(&target->process_list);

    return 0;
}

static void destroy_target_entity(struct target_entity *target)
{
    struct process_entity *process;
    struct process_entity *tmp;

    destroy_target_metadata(&target->meta);

    WARN_ON(!hlist_unhashed(&target->table_node));

    WARN_ON(rwsem_is_locked(&target->action_rwsem));
    WARN_ON(!list_empty(&target->loaded_list));
    WARN_ON(!list_empty(&target->actived_list));
    WARN_ON(!list_empty(&target->func_list));
    INIT_LIST_HEAD(&target->loaded_list);
    INIT_LIST_HEAD(&target->actived_list);
    INIT_LIST_HEAD(&target->func_list);

    mutex_destroy(&target->process_mutex);
    list_for_each_entry_safe(process, tmp, &target->process_list, process_node) {
        list_del_init(&process->process_node);
        free_process(process);
    }
    INIT_LIST_HEAD(&target->process_list);
}

static inline struct target_function *target_get_function(struct target_entity *target, u64 addr)
{
    struct target_function *target_func;

    list_for_each_entry(target_func, &target->func_list, func_node) {
        if (target_func->addr == addr) {
            return target_func;
        }
    }

    return NULL;
}

static inline struct process_entity *target_get_process(struct target_entity *target)
{
    struct pid *current_pid = get_task_pid(current, PIDTYPE_TGID);
    struct process_entity *process;
    struct process_entity *found = NULL;

    list_for_each_entry(process, &target->process_list, process_node) {
        if (pid_nr(process->pid) == pid_nr(current_pid)) {
            found = process;
            break;
        }
    }

    put_pid(current_pid);
    return found;
}

/* public interface */
struct target_entity *new_target_entity(const char *file_path)
{
    struct target_entity *target = NULL;
    int ret;

    if (unlikely(!file_path)) {
        return ERR_PTR(-EINVAL);
    }

    target = kzalloc(sizeof(struct target_entity), GFP_KERNEL);
    if (!target) {
        log_err("failed to alloc target entity\n");
        return ERR_PTR(-ENOMEM);
    }

    ret = resolve_target_entity(target, file_path);
    if (ret) {
        kfree(target);
        return ERR_PTR(ret);
    }

    return target;
}

void free_target_entity(struct target_entity *target)
{
    if (unlikely(!target)) {
        return;
    }

    log_debug("free patch target '%s'\n", target->meta.path);
    destroy_target_entity(target);
    kfree(target);
}

int target_add_function(struct target_entity *target, struct upatch_function *func, bool *need_register)
{
    struct target_function *target_func;

    if (!target || !func || !need_register) {
        return -EINVAL;
    }

    *need_register = false;

    // get or alloc target function
    target_func = target_get_function(target, func->old_addr);
    if (!target_func) {
        target_func = kzalloc(sizeof(*target_func), GFP_KERNEL);
        if (!target_func) {
            log_err("failed to alloc target function\n");
            return -ENOMEM;
        }
        target_func->addr = func->old_addr;
        target_func->count = 0;
        INIT_LIST_HEAD(&target_func->func_node);

        *need_register = true;
        list_add(&target_func->func_node, &target->func_list);
    }

    target_func->count += 1;

    return 0;
}

void target_remove_function(struct target_entity *target, struct upatch_function *func, bool *need_unregister)
{
    struct target_function *target_func;

    if (!target || !func || !need_unregister) {
        return;
    }

    *need_unregister = false;

    target_func = target_get_function(target, func->old_addr);
    if (!target_func) {
        log_warn("target does not have function\n");
        return;
    }

    target_func->count -= 1;
    if (!target_func->count) {
        list_del_init(&target_func->func_node);
        kfree(target_func);
        *need_unregister = true;
    }
}

void target_gather_exited_processes(struct target_entity *target, struct list_head *process_list)
{
    struct process_entity *process;
    struct process_entity *tmp;

    if (unlikely(!target || !process_list)) {
        return;
    }

    list_for_each_entry_safe(process, tmp, &target->process_list, process_node) {
        if (!pid_task(process->pid, PIDTYPE_TGID)) {
            list_move(&process->process_node, process_list);
        }
    }
}

struct process_entity *target_get_or_create_process(struct target_entity *target)
{
    struct process_entity *process = NULL;

    if (unlikely(!target)) {
        return NULL;
    }

    process = target_get_process(target);
    if (!process) {
        log_debug("create process %d for '%s'\n", task_pid_nr(current), target->meta.path);
        process = new_process(target);
        if (IS_ERR(process)) {
            log_err("failed to create target process, ret=%d\n", (int)PTR_ERR(process));
            return NULL;
        }
        list_add(&process->process_node, &target->process_list);
    }

    return process;
}

int target_check_patch_removable(struct target_entity *target, struct patch_entity *patch)
{
    struct process_entity *process = NULL;
    int ret = 0;

    if (unlikely(!target || !patch)) {
        return -EINVAL;
    }

    list_for_each_entry(process, &target->process_list, process_node) {
        ret = process_check_patch_on_stack(process, patch);
        if (ret) {
            break;
        }
    }

    return ret;
}
