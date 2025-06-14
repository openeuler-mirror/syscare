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

static void free_elf_meta(struct target_metadata *meta)
{
    KFREE_CLEAR(meta->file_name);
    VFREE_CLEAR(meta->symtab);
    VFREE_CLEAR(meta->dynsym);
    VFREE_CLEAR(meta->dynamic);
    VFREE_CLEAR(meta->rela_dyn);
    VFREE_CLEAR(meta->rela_plt);
    VFREE_CLEAR(meta->dynstr);
    VFREE_CLEAR(meta->strtab);
}

static int parse_target_load_addr(struct target_metadata *meta, struct file *target)
{
    Elf_Ehdr elf_header;
    Elf_Phdr *phdr = NULL;
    Elf_Addr vma_base_addr = ELF_ADDR_MAX;
    int size;
    int i;
    loff_t pos;
    int ret;

    meta->len = i_size_read(file_inode(target));

    ret = kernel_read(target, &elf_header, sizeof(elf_header), 0);
    if (ret != sizeof(elf_header)) {
        log_err("failed to read elf header, ret=%d\n", ret);
        ret = -ENOEXEC;
        goto out;
    }

    size = sizeof(Elf_Phdr) * elf_header.e_phnum;
    phdr = kmalloc(size, GFP_KERNEL);
    if (!phdr) {
        log_err("failed to kmalloc program headers\n");
        ret = -ENOMEM;
        goto out;
    }

    pos = elf_header.e_phoff;
    ret = kernel_read(target, phdr, size, &pos);
    if (ret < 0) {
        log_err("failed to read program headers, ret=%d\n", ret);
        ret = -ENOEXEC;
        goto out;
    }

    for (i = 0; i < elf_header.e_phnum; i++) {
        if (phdr[i].p_type == PT_LOAD) {
            vma_base_addr = min(vma_base_addr, phdr[i].p_vaddr);
        }
    }

    for (i = 0; i < elf_header.e_phnum; i++) {
        if ((phdr[i].p_type == PT_LOAD) && (phdr[i].p_flags & PF_X)) {
            if (meta->code_vma_offset) {
                log_err("found multiple executable PT_LOAD segments (expected one)\n");
                ret = -ENOEXEC;
                goto out;
            }
            meta->code_vma_offset = (phdr[i].p_vaddr - vma_base_addr) & PAGE_MASK;
            meta->code_virt_offset = phdr[i].p_vaddr - vma_base_addr - phdr[i].p_offset;
        }
    }

    ret = 0;
out:
    KFREE_CLEAR(phdr);
    return ret;
}

static int process_section_header(struct file *target, Elf_Shdr *shdr, char *sh_name, struct target_metadata *meta)
{
    void *sh_data = NULL;
    int ret = 0;

    if (shdr->sh_type == SHT_SYMTAB) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->symtab = sh_data;
        meta->num.symtab = shdr->sh_size / sizeof(Elf_Sym);
    } else if (strcmp(sh_name, STRTAB_NAME) == 0) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->strtab = sh_data;
    } else if (strcmp(sh_name, DYNSTR_NAME) == 0) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->dynstr = sh_data;
    } else if (strcmp(sh_name, DYN_RELA_NAME) == 0 || strcmp(sh_name, DYN_REL_NAME) == 0) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->rela_dyn = sh_data;
        meta->num.rela_dyn = shdr->sh_size / sizeof(Elf_Rela);
    } else if (strcmp(sh_name, PLT_RELA_NAME) == 0 || strcmp(sh_name, PLT_REL_NAME) == 0) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->rela_plt = sh_data;
        meta->num.rela_plt = shdr->sh_size / sizeof(Elf_Rela);
    } else if (shdr->sh_type == SHT_DYNAMIC) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->dynamic = sh_data;
    } else if (shdr->sh_type == SHT_DYNSYM) {
        sh_data = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
        meta->dynsym = sh_data;
        meta->num.dynsym = shdr->sh_size / sizeof(Elf_Sym);
    }

    if (IS_ERR_VALUE(sh_data)) {
        ret = PTR_ERR(sh_data);
        log_err("failed to read section '%s'\n", sh_name);
    }
    return ret;
}

static int init_target_meta(struct target_metadata *meta, struct file *target)
{
    int ret;

    Elf_Ehdr *ehdr = NULL;  // elf header
    Elf_Shdr *shdrs = NULL; // section headers
    char *shstrtab = NULL;  // section string table
    Elf_Phdr *phdrs = NULL;

    Elf_Shdr *shdr;
    Elf_Half i;
    char *sh_name;

    const unsigned char *base_name;

    meta->file_name = NULL;

    // Check if target dentry are valid before accessing
    if (!target->f_path.dentry) {
        log_err("Invalid target file pointer or dentry\n");
        ret = -EINVAL;
        goto out;
    }

    base_name = target->f_path.dentry->d_name.name;

    meta->file_name = kstrdup(base_name, GFP_KERNEL);
    if (!meta->file_name) {
        log_err("Failed to allocate memory for filename\n");
        ret = -ENOMEM;
        goto out;
    }

    ret = parse_target_load_addr(meta, target);
    if (ret) {
        goto out;
    }

    // read elf header
    ret = kernel_read(target, &meta->ehdr, sizeof(Elf_Ehdr), 0);
    if (ret != sizeof(Elf_Ehdr)) {
        ret = -ENOEXEC;
        log_err("read elf header failed ret=%d\n", ret);
        goto out;
    }

    ehdr = &meta->ehdr;
    if (!is_elf_valid(ehdr, i_size_read(file_inode(target)), false)) {
        ret = -ENOEXEC;
        log_err("invalid target format\n");
        goto out;
    }

    // read section headers
    shdrs = vmalloc_read(target, ehdr->e_shoff, ehdr->e_shentsize * ehdr->e_shnum);
    if (IS_ERR(shdrs)) {
        ret = PTR_ERR(shdrs);
        log_err("failed to read section header\n");
        goto out;
    }

    // read section header string table
    shdr = &shdrs[ehdr->e_shstrndx];
    shstrtab = vmalloc_read(target, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(shstrtab)) {
        ret = PTR_ERR(shstrtab);
        log_err("failed to read '%s' section\n", SHSTRTAB_NAME);
        goto out;
    }

    // resolve section headers
    for (i = 1; i < ehdr->e_shnum; i++) {
        shdr = &shdrs[i];
        sh_name = shstrtab + shdr->sh_name;

        ret = process_section_header(target, shdr, sh_name, meta);
        if (ret)
            goto out;
    }

    phdrs = vmalloc_read(target, ehdr->e_phoff, ehdr->e_phentsize * ehdr->e_phnum);
    if (IS_ERR(phdrs)) {
        ret = PTR_ERR(phdrs);
        log_err("failed to read program header\n");
        goto out;
    }

    for (i = 0; i < ehdr->e_phnum; i++) {
        if (phdrs[i].p_type == PT_TLS) {
            meta->tls_size = phdrs[i].p_memsz;
            meta->tls_align = phdrs[i].p_align;
            log_debug("Found TLS size = %zd, align = %zd\n",
                (size_t)meta->tls_size, (size_t)meta->tls_align);
            break;
        }
    }
    ret = 0;

out:
    if (ret != 0) {
        free_elf_meta(meta);
    }
    VFREE_CLEAR(shdrs);
    VFREE_CLEAR(shstrtab);
    VFREE_CLEAR(phdrs);
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

    // resolve elf meta
    ret = init_target_meta(&target->meta, file);
    if (ret != 0) {
        iput(target->inode);
        KFREE_CLEAR(target->path);
        log_err("failed to resolve elf meta\n");
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
    free_elf_meta(&target->meta);
    hash_del(&target->node);

    target_unregister_uprobes(target);

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
