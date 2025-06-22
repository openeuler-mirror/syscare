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
#include "patch_manage.h"
#include "util.h"

#define TARGET_TABLE_HASH_BITS 4
#define ELF_ADDR_MAX UINT_MAX

static const char *SHSTRTAB_NAME = ".shstrtab";
static const char *STRTAB_NAME   = ".strtab";
static const char *DYNSTR_NAME   = ".dynstr";
static const char *DYN_RELA_NAME = ".rela.dyn";
static const char *PLT_RELA_NAME = ".rela.plt";
static const char *PLT_NAME      = ".plt";
static const char *GOT_NAME      = ".got";

DEFINE_HASHTABLE(g_target_table, TARGET_TABLE_HASH_BITS);
DEFINE_MUTEX(g_target_table_lock);

static void clear_target_metadata(struct target_metadata *meta)
{
    KFREE_CLEAR(meta->file_name);

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
}

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

    Elf_Phdr *phdrs = meta->phdrs;
    Elf_Half phdr_num = meta->ehdr->e_phnum;
    Elf_Phdr *phdr;

    Elf_Shdr *shdrs = meta->shdrs;
    Elf_Half shdr_num = meta->ehdr->e_shnum;
    Elf_Shdr *shdr;

    const char *shstrtab = meta->shstrtab;
    const char *sec_name;

    Elf_Half i;

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

static int resolve_target_metadata(struct target_metadata *meta, struct file *target)
{
    int ret = 0;

    if (!target->f_path.dentry) {
        log_err("invalid file dentry\n");
        ret = -EINVAL;
        goto out;
    }

    meta->file_name = kstrdup(target->f_path.dentry->d_name.name, GFP_KERNEL);
    if (!meta->file_name) {
        log_err("failed to alloc filename\n");
        ret = -ENOMEM;
        goto out;
    }
    meta->file_size = i_size_read(file_inode(target));

    meta->ehdr = vmalloc_read(target, 0, sizeof(Elf_Ehdr));
    if (IS_ERR(meta->ehdr)) {
        ret = PTR_ERR(meta->ehdr);
        log_err("failed to read elf header\n");
        goto out;
    }

    if (!is_valid_target(meta->ehdr, meta->file_size)) {
        ret = -ENOEXEC;
        log_err("invalid file format\n");
        goto out;
    }

    meta->phdrs = vmalloc_read(target, meta->ehdr->e_phoff, meta->ehdr->e_phentsize * meta->ehdr->e_phnum);
    if (IS_ERR(meta->phdrs)) {
        ret = PTR_ERR(meta->phdrs);
        log_err("failed to read program header\n");
        goto out;
    }

    meta->shdrs = vmalloc_read(target, meta->ehdr->e_shoff, meta->ehdr->e_shentsize * meta->ehdr->e_shnum);
    if (IS_ERR(meta->shdrs)) {
        ret = PTR_ERR(meta->shdrs);
        log_err("failed to read section header\n");
        goto out;
    }

    ret = resolve_target_sections(meta, target);
    if (ret) {
        log_err("failed to resolve target sections\n");
        goto out;
    }

    ret = resolve_target_address(meta);
    if (ret) {
        log_err("failed to resolve target address\n");
        goto out;
    }

    return 0;

out:
    clear_target_metadata(meta);
    return ret;
}

static int resolve_target_entity(struct target_entity *target, const char *file_path)
{
    int ret = 0;
    struct file *file = NULL;

    init_rwsem(&target->patch_lock);
    mutex_init(&target->process_lock);
    INIT_HLIST_NODE(&target->node);
    INIT_LIST_HEAD(&target->offset_node);
    INIT_LIST_HEAD(&target->all_patch_list);
    INIT_LIST_HEAD(&target->actived_patch_list);
    INIT_LIST_HEAD(&target->process_head);

    file = filp_open(file_path, O_RDONLY, 0); // open file by inode
    if (IS_ERR(file)) {
        log_err("failed to open '%s'\n", file_path);
        return PTR_ERR(file);
    }

    target->inode = igrab(file_inode(file));
    if (!target->inode) {
        log_err("file '%s' inode is invalid\n", file_path);
        ret = -ENOENT;
        goto out;
    }

    target->path = kstrdup(file_path, GFP_KERNEL);
    if (!target->path) {
        iput(target->inode);
        ret = -ENOMEM;
        log_err("faild to alloc filename\n");
        goto out;
    }

    ret = resolve_target_metadata(&target->meta, file);
    if (ret != 0) {
        iput(target->inode);
        KFREE_CLEAR(target->path);
        goto out;
    }

out:
    filp_close(file, NULL);
    return ret;
}

struct target_entity *get_target_entity_by_inode(struct inode *inode)
{
    struct target_entity *target;
    struct target_entity *found = NULL;

    mutex_lock(&g_target_table_lock);
    hash_for_each_possible(g_target_table, target, node, inode->i_ino) {
        if (target->inode == inode) {
            found = target;
            break;
        }
    }
    mutex_unlock(&g_target_table_lock);
    return found;
}

/* public interface */
struct target_entity *get_target_entity(const char *path)
{
    struct target_entity *target;
    struct inode *inode;

    inode = get_path_inode(path);
    if (!inode) {
        log_err("failed to get '%s' inode\n", path);
        return NULL;
    }

    target = get_target_entity_by_inode(inode);
    iput(inode);

    return target;
}

static void insert_target(struct target_entity *target)
{
    mutex_lock(&g_target_table_lock);
    hash_add(g_target_table, &target->node, target->inode->i_ino);
    mutex_unlock(&g_target_table_lock);
}

struct target_entity *new_target_entity(const char *file_path)
{
    struct target_entity *target = NULL;
    int ret = 0;

    log_debug("create patch target entity '%s'\n", file_path);

    if (!file_path) {
        return ERR_PTR(-EINVAL);
    }

    target = kzalloc(sizeof(struct target_entity), GFP_KERNEL);
    if (!target) {
        log_err("failed to alloc target entity\n");
        return ERR_PTR(-ENOMEM);
    }

    ret = resolve_target_entity(target, file_path);
    if (ret != 0) {
        kfree(target);
        return ERR_PTR(ret);
    }

    insert_target(target);
    return target;
}

// caller should lock g_target_table_lock
void free_target_entity(struct target_entity *target)
{
    struct process_entity *process;
    struct process_entity *tmp_pro;
    struct patch_entity *patch;
    struct patched_offset *off;

    log_debug("free patch target '%s'\n", target->path);
    down_write(&target->patch_lock);

    list_for_each_entry(off, &target->offset_node, list) {
        log_err("found uprobe in 0x%lx\n", (unsigned long)off->offset);
    }

    list_for_each_entry(patch, &target->actived_patch_list, actived_node) {
        log_err("found actived patch '%s'\n", patch->path ? patch->path : "NULL");
    }

    list_for_each_entry(patch, &target->all_patch_list, patch_node) {
        log_err("found patch '%s'\n", patch->path ? patch->path : "NULL");
    }

    mutex_lock(&target->process_lock);
    list_for_each_entry_safe(process, tmp_pro, &target->process_head, list) {
        free_process(process);
    }
    mutex_unlock(&target->process_lock);

    iput(target->inode);
    KFREE_CLEAR(target->path);
    clear_target_metadata(&target->meta);
    hash_del(&target->node);

    target_unregister_uprobes(target);

    up_write(&target->patch_lock);

    kfree(target);
}

bool is_target_has_patch(const struct target_entity *target)
{
    return !list_empty(&target->all_patch_list);
}

void __exit report_target_table_populated(void)
{
    struct target_entity *target;
    int bkt;

    mutex_lock(&g_target_table_lock);
    hash_for_each(g_target_table, bkt, target, node) {
        log_err("found target '%s' on exit", target->path ? target->path : "(null)");
    }
    mutex_unlock(&g_target_table_lock);
}
