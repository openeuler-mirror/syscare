// SPDX-License-Identifier: GPL-2.0
/*
 * maintain patch info
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

#include "patch_entity.h"

#include <linux/fs.h>
#include <linux/hashtable.h>

#include "patch_load.h"
#include "util.h"

#define PATCH_TABLE_HASH_BITS 4

static const char *SYMTAB_NAME    = ".symtab";
static const char *TEXT_RELA_NAME = ".rela.text.";

static const char *UPATCH_FUNCS_NAME      = ".upatch.funcs";
static const char *UPATCH_FUNCS_RELA_NAME = ".rela.upatch.funcs";
static const char *UPATCH_STRINGS_NAME    = ".upatch.strings";

DEFINE_HASHTABLE(g_patch_table, PATCH_TABLE_HASH_BITS);
DEFINE_MUTEX(g_patch_table_lock);

static inline void count_und_symbol(struct patch_metadata *meta, Elf_Sym *symtab, size_t count)
{
    size_t i;

    for (i = 1; i < count; i++) {
        if (symtab[i].st_shndx == SHN_UNDEF) {
            meta->und_sym_num++;
        }
    }
}

static inline void count_got_reloc(struct patch_metadata *meta, Elf_Rela *relas, size_t count)
{
    size_t i;

    for (i = 0; i < count; i++) {
        if (is_got_rela_type(ELF_R_TYPE(relas[i].r_info))) {
            meta->got_reloc_num++;
        }
    }
}

static void clear_patch_metadata(struct patch_metadata *meta)
{
    VFREE_CLEAR(meta->file_buff);
    meta->file_size = 0;

    meta->symtab_index = 0;
    meta->strtab_index = 0;

    meta->funcs = NULL;
    meta->strings = NULL;

    meta->func_num = 0;
    meta->string_len = 0;

    meta->und_sym_num = 0;
    meta->got_reloc_num = 0;
}

static int resolve_patch_metadata(struct patch_metadata *meta, struct file *patch)
{
    int ret = 0;

    loff_t file_size;

    Elf_Ehdr *ehdr;
    Elf_Shdr *shdrs;
    Elf_Half shdr_num;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab;
    size_t shstrtab_size;

    const char *sec_name;
    void *sec_data;

    struct upatch_relocation *relas = NULL;
    size_t rela_num = 0;

    meta->file_size = i_size_read(file_inode(patch));
    meta->file_buff = vmalloc_read(patch, 0, meta->file_size);
    if (IS_ERR(meta->file_buff)) {
        ret = PTR_ERR(meta->file_buff);
        log_err("failed to read file, len=0x%llx\n", meta->file_size);
        goto fail;
    }

    ehdr = meta->file_buff;
    if (!is_valid_patch(ehdr, meta->file_size)) {
        ret = -ENOEXEC;
        log_err("invalid file format\n");
        goto fail;
    }
    meta->shstrtab_index = ehdr->e_shstrndx;

    file_size = meta->file_size;
    shdrs = meta->file_buff + ehdr->e_shoff;
    shdr_num = ehdr->e_shnum;
    shstrtab = meta->file_buff + shdrs[meta->shstrtab_index].sh_offset;
    shstrtab_size = shdrs[meta->shstrtab_index].sh_size;

    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        sec_name = get_string_at(shstrtab, shstrtab_size, shdr->sh_name);
        if (sec_name == NULL) {
            ret = -ENOEXEC;
            log_err("invalid section name, index=%u\n", i);
            goto fail;
        }

        sec_name = shstrtab + shdr->sh_name; // no need check
        if (shdr->sh_type != SHT_NOBITS && shdr->sh_offset + shdr->sh_size > file_size) {
            log_err("section '%s' offset overflow, index=%u\n", sec_name, i);
            ret = -ENOEXEC;
            goto fail;
        }

        sec_data = meta->file_buff + shdr->sh_offset;
        switch (shdr->sh_type) {
            case SHT_PROGBITS:
                if (strcmp(sec_name, UPATCH_FUNCS_NAME) == 0) {
                    meta->func_index = i;
                    meta->funcs = sec_data;
                    meta->func_num = shdr->sh_size / sizeof(struct upatch_function);
                } else if (strcmp(sec_name, UPATCH_STRINGS_NAME) == 0) { // .upatch.strings is not SHT_STRTAB
                    meta->string_index = i;
                    meta->strings = sec_data;
                    meta->string_len = shdr->sh_size;
                }
                break;

            case SHT_SYMTAB:
                if (shdr->sh_entsize != sizeof(Elf_Sym)) {
                    log_err("invalid section '%s' entity size\n", sec_name);
                    ret = -ENOEXEC;
                    goto fail;
                }
                if (shdr->sh_link > shdr_num) {
                    ret = -ENOEXEC;
                    log_err("invalid section '%s' string table index\n", sec_name);
                    goto fail;
                }
                if (strcmp(sec_name, SYMTAB_NAME) == 0) {
                    meta->symtab_index = i;
                    meta->strtab_index = shdr->sh_link;
                    count_und_symbol(meta, sec_data, shdr->sh_size / shdr->sh_entsize);
                }
                break;

            case SHT_RELA:
                if (shdr->sh_entsize != sizeof(Elf_Rela)) {
                    log_err("invalid section '%s' entity size\n", sec_name);
                    ret = -ENOEXEC;
                    goto fail;
                }
                if (strcmp(sec_name, UPATCH_FUNCS_RELA_NAME) == 0) {
                    meta->rela_index = i;
                    relas = sec_data;
                    rela_num = shdr->sh_size / sizeof(struct upatch_relocation);
                } else if (strncmp(sec_name, TEXT_RELA_NAME, strlen(TEXT_RELA_NAME)) == 0) {
                    count_got_reloc(meta, sec_data, shdr->sh_size / shdr->sh_entsize);
                }
                break;

            default:
                break;
        }
    }

    if (!meta->symtab_index || !meta->strtab_index) {
        log_err("patch contains no symbol\n");
        ret = -ENOEXEC;
        goto fail;
    }
    if (!meta->func_index || !meta->funcs || !meta->func_num) {
        log_err("patch contains no function\n");
        ret = -ENOEXEC;
        goto fail;
    }
    if (!meta->rela_index || !meta->string_index ||
        !relas || !meta->strings ||
        !rela_num || !meta->string_len ||
        meta->func_num != rela_num) {
        log_err("invalid patch format\n");
        ret = -ENOEXEC;
        goto fail;
    }

    for (i = 0; i < rela_num; i++) {
        meta->funcs[i].name_off = relas[i].name.r_addend;
    }

    return 0;

fail:
    clear_patch_metadata(meta);
    return ret;
}

static int resolve_patch_entity(struct patch_entity *patch, const char *file_path)
{
    int ret = 0;

    struct file *file;

    INIT_HLIST_NODE(&patch->node);
    INIT_LIST_HEAD(&patch->patch_node);
    INIT_LIST_HEAD(&patch->actived_node);

    file = filp_open(file_path, O_RDONLY, 0);
    if (IS_ERR(file)) {
        log_err("failed to open '%s'\n", file_path);
        return PTR_ERR(file);
    }

    patch->inode = igrab(file_inode(file));
    if (!patch->inode) {
        log_err("file '%s' inode is invalid\n", file_path);
        ret = -ENOENT;
        goto out;
    }

    patch->path = kstrdup(file_path, GFP_KERNEL);
    if (!patch->path) {
        ret = -ENOMEM;
        iput(patch->inode);
        log_err("faild to alloc filename\n");
        goto out;
    }

    ret = resolve_patch_metadata(&patch->meta, file);
    if (ret) {
        iput(patch->inode);
        KFREE_CLEAR(patch->path);
        goto out;
    }

    patch->status = UPATCH_STATUS_DEACTIVED;

out:
    filp_close(file, NULL);
    return ret;
}

static struct patch_entity *get_patch_entity_by_inode(struct inode *inode)
{
    struct patch_entity *patch;
    struct patch_entity *found = NULL;

    mutex_lock(&g_patch_table_lock);
    hash_for_each_possible(g_patch_table, patch, node, inode->i_ino) {
        if (patch->inode == inode) {
            found = patch;
            break;
        }
    }
    mutex_unlock(&g_patch_table_lock);
    return found;
}

/* public interface */
struct patch_entity *get_patch_entity(const char *path)
{
    struct patch_entity *patch;
    struct inode *inode;

    inode = get_path_inode(path);
    if (!inode) {
        log_err("failed to get '%s' inode\n", path);
        return NULL;
    }

    patch = get_patch_entity_by_inode(inode);
    iput(inode);

    return patch;
}

struct patch_entity *new_patch_entity(const char *file_path)
{
    int ret = 0;
    struct patch_entity *patch = NULL;

    patch = kzalloc(sizeof(struct patch_entity), GFP_KERNEL);
    if (!patch) {
        log_err("failed to alloc patch entity\n");
        return ERR_PTR(-ENOMEM);
    }

    ret = resolve_patch_entity(patch, file_path);
    if (ret) {
        kfree(patch);
        return ERR_PTR(ret);
    }

    mutex_lock(&g_patch_table_lock);
    hash_add(g_patch_table, &patch->node, patch->inode->i_ino);
    mutex_unlock(&g_patch_table_lock);

    return patch;
}

void free_patch_entity(struct patch_entity *patch)
{
    if (!patch) {
        return;
    }

    log_debug("free patch '%s'\n", patch->path);

    iput(patch->inode);
    KFREE_CLEAR(patch->path);
    clear_patch_metadata(&patch->meta);

    list_del(&patch->actived_node);
    list_del(&patch->patch_node);
    hash_del(&patch->node);

    kfree(patch);
}

void __exit report_patch_table_populated(void)
{
    struct patch_entity *patch;
    int bkt;

    mutex_lock(&g_patch_table_lock);
    hash_for_each(g_patch_table, bkt, patch, node) {
        log_warn("found patch '%s' (%s) on exit",
            patch->path ? patch->path : "(null)", patch_status_str(patch->status));
    }
    mutex_unlock(&g_patch_table_lock);
}
