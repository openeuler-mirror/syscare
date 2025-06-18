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

#define SHSTRTAB_NAME   ".shstrtab"
#define STRTAB_NAME     ".strtab"
#define DYNSTR_NAME     ".dynstr"
#define DYN_RELA_NAME   ".rela.dyn"
#define PLT_RELA_NAME   ".rela.plt"
#define DYN_REL_NAME    ".rel.dyn"
#define PLT_REL_NAME    ".rel.plt"

DEFINE_HASHTABLE(g_targets, TARGETS_HASH_BITS);
DEFINE_MUTEX(g_target_table_lock);

static void destroy_target_metadata(struct target_metadata *meta)
{
    KFREE_CLEAR(meta->file_name);

    VFREE_CLEAR(meta->ehdr);
    VFREE_CLEAR(meta->phdrs);
    VFREE_CLEAR(meta->shdrs);
    VFREE_CLEAR(meta->shstrtab);

    VFREE_CLEAR(meta->symtab);
    VFREE_CLEAR(meta->dynsym);
    VFREE_CLEAR(meta->dynamic);
    VFREE_CLEAR(meta->rela_dyn);
    VFREE_CLEAR(meta->rela_plt);
    VFREE_CLEAR(meta->dynstr);
    VFREE_CLEAR(meta->strtab);
}

static int parse_target_address(struct target_metadata *meta)
{
    int ret = 0;
    Elf_Addr first_load_addr = ELF_ADDR_MAX;

    for (Elf_Half i = 0; i < meta->ehdr->e_phnum; i++) {
        if (meta->phdrs[i].p_type == PT_LOAD) {
            first_load_addr = min(first_load_addr, meta->phdrs[i].p_vaddr);
        }
    }

    for (Elf_Half i = 0; i < meta->ehdr->e_phnum; i++) {
        if (meta->phdrs[i].p_type == PT_TLS) {
            meta->tls_size = meta->phdrs[i].p_memsz;
            meta->tls_align = meta->phdrs[i].p_align;
        }
        if ((meta->phdrs[i].p_type == PT_LOAD) && (meta->phdrs[i].p_flags & PF_X)) {
            if (meta->code_vma_offset) {
                log_err("found multiple executable PT_LOAD segments (expected one)\n");
                ret = -ENOEXEC;
                goto out;
            }
            meta->code_vma_offset = (meta->phdrs[i].p_vaddr - first_load_addr) & PAGE_MASK;
            meta->code_virt_offset = meta->phdrs[i].p_vaddr - first_load_addr - meta->phdrs[i].p_offset;
        }
    }

out:
    return ret;
}

static int parse_target_sections(struct target_metadata *meta, struct file *target)
{
    int ret = 0;
    Elf_Shdr *shdr = NULL;
    char *sh_name = NULL;
    void *sh_data = NULL;

    shdr = &meta->shdrs[meta->ehdr->e_shstrndx];
    meta->shstrtab = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(meta->shstrtab)) {
        ret = PTR_ERR(meta->shstrtab);
        log_err("failed to read section '%s'\n", SHSTRTAB_NAME);
        goto out;
    }

    for (Elf_Half i = 1; i < meta->ehdr->e_shnum; i++) {
        shdr = &meta->shdrs[i];
        sh_name = meta->shstrtab + shdr->sh_name;

        switch (shdr->sh_type) {
            case SHT_SYMTAB:
                sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->symtab = sh_data;
                meta->num.symtab = shdr->sh_size / sizeof(Elf_Sym);
                break;

            case SHT_STRTAB:
                if (strcmp(sh_name, STRTAB_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->strtab = sh_data;
                } else if (strcmp(sh_name, DYNSTR_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->dynstr = sh_data;
                }
                break;

            case SHT_REL:
                if (strcmp(sh_name, DYN_REL_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_dyn = sh_data;
                    meta->num.rela_dyn = shdr->sh_size / sizeof(Elf_Rela);
                } else if (strcmp(sh_name, PLT_REL_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_plt = sh_data;
                    meta->num.rela_plt = shdr->sh_size / sizeof(Elf_Rela);
                }
                break;

            case SHT_RELA:
                if (strcmp(sh_name, DYN_RELA_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_dyn = sh_data;
                    meta->num.rela_dyn = shdr->sh_size / sizeof(Elf_Rela);
                } else if (strcmp(sh_name, PLT_RELA_NAME) == 0) {
                    sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                    meta->rela_plt = sh_data;
                    meta->num.rela_plt = shdr->sh_size / sizeof(Elf_Rela);
                }
                break;

            case SHT_DYNAMIC:
                sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->dynamic = sh_data;
                break;

            case SHT_DYNSYM:
                sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
                meta->dynsym = sh_data;
                meta->num.dynsym = shdr->sh_size / sizeof(Elf_Sym);
                break;

            default:
                // do nothing
                break;
        }

        if (IS_ERR(sh_data)) {
            ret = PTR_ERR(sh_data);
            log_err("failed to read section '%s'\n", sh_name);
            break;
        }
    }

out:
    return ret;
}

static int parse_target_metadata(struct target_metadata *meta, struct file *target)
{
    int ret = 0;

    // check if target dentry are valid before accessing
    if (!target->f_path.dentry) {
        log_err("invalid target file dentry\n");
        ret = -EINVAL;
        goto out;
    }

    // parse file info
    meta->file_name = kstrdup(target->f_path.dentry->d_name.name, GFP_KERNEL);
    if (!meta->file_name) {
        log_err("failed to alloc filename\n");
        ret = -ENOMEM;
        goto out;
    }
    meta->file_len = i_size_read(file_inode(target));

    // read elf header
    meta->ehdr = vmalloc_read(target, 0, sizeof(Elf_Ehdr));
    if (IS_ERR(meta->ehdr)) {
        ret = PTR_ERR(meta->ehdr);
        log_err("failed to read elf header\n");
        goto out;
    }

    // validate target elf format
    if (!is_elf_valid(meta->ehdr, meta->file_len, false)) {
        ret = -ENOEXEC;
        log_err("invalid file format\n");
        goto out;
    }

    // read section headers
    meta->phdrs = vmalloc_read(target, meta->ehdr->e_phoff, meta->ehdr->e_phentsize * meta->ehdr->e_phnum);
    if (IS_ERR(meta->phdrs)) {
        ret = PTR_ERR(meta->phdrs);
        log_err("failed to read program header");
        goto out;
    }

    // read section headers
    meta->shdrs = vmalloc_read(target, meta->ehdr->e_shoff, meta->ehdr->e_shentsize * meta->ehdr->e_shnum);
    if (IS_ERR(meta->shdrs)) {
        ret = PTR_ERR(meta->shdrs);
        log_err("failed to read section header\n");
        goto out;
    }

    ret = parse_target_sections(meta, target);
    if (ret) {
        log_err("failed to parse target sections\n");
        goto out;
    }

    ret = parse_target_address(meta);
    if (ret) {
        log_err("failed to parse target address\n");
        goto out;
    }

out:
    if (ret != 0) {
        destroy_target_metadata(meta);
    }
    return ret;
}

static int init_grab_target(struct target_entity *target, const char *file_path)
{
    int ret = 0;
    struct file *file = NULL;

    init_rwsem(&target->patch_lock);
    mutex_init(&target->process_lock);
    INIT_HLIST_NODE(&target->node);
    INIT_LIST_HEAD(&target->off_head);
    INIT_LIST_HEAD(&target->all_patch_list);
    INIT_LIST_HEAD(&target->actived_patch_list);
    INIT_LIST_HEAD(&target->process_head);

    // open target file
    file = filp_open(file_path, O_RDONLY, 0); // open file by inode
    if (IS_ERR(file)) {
        log_err("failed to open file '%s'\n", file_path);
        return PTR_ERR(file);
    }

    target->inode = igrab(file_inode(file));
    if (!target->inode) {
        pr_err("%s: Failed to grab inode of %s\n", __func__, file_path);
        ret = -ENOENT;
        goto fail;
    }

    target->path = kstrdup(file_path, GFP_KERNEL);
    if (!target->path) {
        iput(target->inode);
        ret = -ENOMEM;
        goto fail;
    }

    ret = parse_target_metadata(&target->meta, file);
    if (ret != 0) {
        iput(target->inode);
        KFREE_CLEAR(target->path);
        log_err("failed to parse target metadata\n");
        goto fail;
    }

fail:
    filp_close(file, NULL);
    return ret;
}

struct target_entity *get_target_entity_from_inode(struct inode *inode)
{
    struct target_entity *target;
    struct target_entity *found = NULL;

    mutex_lock(&g_target_table_lock);
    hash_for_each_possible(g_targets, target, node, inode->i_ino) {
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
    struct inode *inode = path_inode(path);
    struct target_entity *target;

    log_debug("start to get target_entity for %s\n", path);

    if (IS_ERR(inode)) {
        return NULL;
    }

    inode = igrab(inode);
    if (!inode) {
        pr_err("failed to grab inode of %s\n", path);
        return NULL;
    }

    target = get_target_entity_from_inode(inode);
    iput(inode);
    return target;
}

static void insert_target(struct target_entity *target)
{
    mutex_lock(&g_target_table_lock);
    hash_add(g_targets, &target->node, target->inode->i_ino);
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

    ret = init_grab_target(target, file_path);
    if (ret != 0) {
        log_err("failed to init patch target '%s', ret=%d\n", file_path, ret);
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

    list_for_each_entry(off, &target->off_head, list) {
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
    destroy_target_metadata(&target->meta);
    hash_del(&target->node);

    up_write(&target->patch_lock);

    kfree(target);
}

bool is_target_has_patch(const struct target_entity *target)
{
    return !list_empty(&target->all_patch_list);
}

bool upatch_binary_has_addr(const struct target_entity *target, loff_t offset)
{
    struct patched_offset *addr = NULL;

    if (!target) {
        return false;
    }

    list_for_each_entry(addr, &target->off_head, list) {
        if (addr->offset == offset) {
            return true;
        }
    }

    return false;
}

void __exit verify_target_empty_on_exit(void)
{
    struct target_entity *target;
    int bkt;

    mutex_lock(&g_target_table_lock);
    hash_for_each(g_targets, bkt, target, node) {
        log_err("found target '%s' on exit", target->path ? target->path : "(null)");
    }
    mutex_unlock(&g_target_table_lock);
}
