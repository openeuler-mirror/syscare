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

#include <linux/list.h>
#include <linux/fs.h>

#include "patch_load.h"
#include "util.h"

static const char *SYMTAB_NAME    = ".symtab";
static const char *TEXT_RELA_NAME = ".rela.text.";

static const char *UPATCH_FUNCS_NAME      = ".upatch.funcs";
static const char *UPATCH_FUNCS_RELA_NAME = ".rela.upatch.funcs";
static const char *UPATCH_STRINGS_NAME    = ".upatch.strings";

/* --- Patch life-cycle management --- */

static void destroy_patch_file(struct patch_file *patch_file)
{
    iput(patch_file->inode);

    patch_file->path = NULL;
    patch_file->inode = NULL;
    patch_file->size = 0;

    VFREE_CLEAR(patch_file->buff);

    patch_file->shstrtab_index = 0;
    patch_file->symtab_index = 0;
    patch_file->strtab_index = 0;

    patch_file->func_index = 0;
    patch_file->rela_index = 0;
    patch_file->string_index = 0;

    patch_file->funcs = NULL;
    patch_file->strings = NULL;

    patch_file->func_num = 0;
    patch_file->string_len = 0;

    patch_file->und_sym_num = 0;
    patch_file->got_reloc_num = 0;
}

static inline void resolve_und_symbol_count(struct patch_file *patch_file, Elf_Sym *symtab, size_t count)
{
    size_t i;

    for (i = 1; i < count; i++) {
        if (symtab[i].st_shndx == SHN_UNDEF) {
            patch_file->und_sym_num++;
        }
    }
}

static inline void resolve_got_reloc_count(struct patch_file *patch_file, Elf_Rela *relas, size_t count)
{
    size_t i;

    for (i = 0; i < count; i++) {
        if (is_got_rela_type(ELF_R_TYPE(relas[i].r_info))) {
            patch_file->got_reloc_num++;
        }
    }
}

static int resolve_patch_file(struct patch_file *patch_file, struct file *file)
{
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

    int ret = 0;

    rcu_read_lock();
    patch_file->path = file_path(file, patch_file->path_buff, PATH_MAX);
    rcu_read_unlock();

    if (unlikely(IS_ERR(patch_file->path))) {
        ret = PTR_ERR(patch_file->path);
        log_err("faild to get file path\n");
        goto out;
    }

    patch_file->inode = igrab(file_inode(file));
    if (unlikely(!patch_file->inode)) {
        log_err("failed to get file inode\n");
        ret = -ENOENT;
        goto out;
    }

    patch_file->size = i_size_read(patch_file->inode);

    patch_file->buff = vmalloc_read(file, 0, patch_file->size);
    if (unlikely(IS_ERR(patch_file->buff))) {
        log_err("failed to read file, len=0x%llx\n", patch_file->size);
        ret = PTR_ERR(patch_file->buff);
        goto out;
    }

    ehdr = patch_file->buff;
    if (unlikely(!is_valid_patch(ehdr, patch_file->size))) {
        log_err("invalid file format\n");
        ret = -ENOEXEC;
        goto out;
    }
    patch_file->shstrtab_index = ehdr->e_shstrndx;

    shdrs = patch_file->buff + ehdr->e_shoff;
    shdr_num = ehdr->e_shnum;
    shstrtab = patch_file->buff + shdrs[patch_file->shstrtab_index].sh_offset;
    shstrtab_size = shdrs[patch_file->shstrtab_index].sh_size;

    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        sec_name = get_string_at(shstrtab, shstrtab_size, shdr->sh_name);
        if (unlikely(sec_name == NULL)) {
            log_err("invalid section name, index=%u\n", i);
            ret = -ENOEXEC;
            goto out;
        }

        sec_name = shstrtab + shdr->sh_name; // no need check
        if (unlikely(shdr->sh_type != SHT_NOBITS && shdr->sh_offset + shdr->sh_size > patch_file->size)) {
            log_err("section '%s' offset overflow, index=%u\n", sec_name, i);
            ret = -ENOEXEC;
            goto out;
        }

        sec_data = patch_file->buff + shdr->sh_offset;
        switch (shdr->sh_type) {
            case SHT_PROGBITS:
                if (strcmp(sec_name, UPATCH_FUNCS_NAME) == 0) {
                    patch_file->func_index = i;
                    patch_file->funcs = sec_data;
                    patch_file->func_num = shdr->sh_size / sizeof(struct upatch_function);
                } else if (strcmp(sec_name, UPATCH_STRINGS_NAME) == 0) { // .upatch.strings is not SHT_STRTAB
                    patch_file->string_index = i;
                    patch_file->strings = sec_data;
                    patch_file->string_len = shdr->sh_size;
                }
                break;

            case SHT_SYMTAB:
                if (unlikely(shdr->sh_entsize != sizeof(Elf_Sym))) {
                    log_err("invalid section '%s' entity size\n", sec_name);
                    ret = -ENOEXEC;
                    goto out;
                }
                if (unlikely(shdr->sh_link > shdr_num)) {
                    log_err("invalid section '%s' string table index\n", sec_name);
                    ret = -ENOEXEC;
                    goto out;
                }
                if (strcmp(sec_name, SYMTAB_NAME) == 0) {
                    patch_file->symtab_index = i;
                    patch_file->strtab_index = shdr->sh_link;
                    resolve_und_symbol_count(patch_file, sec_data, shdr->sh_size / shdr->sh_entsize);
                }
                break;

            case SHT_RELA:
                if (unlikely(shdr->sh_entsize != sizeof(Elf_Rela))) {
                    log_err("invalid section '%s' entity size\n", sec_name);
                    ret = -ENOEXEC;
                    goto out;
                }
                if (strcmp(sec_name, UPATCH_FUNCS_RELA_NAME) == 0) {
                    patch_file->rela_index = i;
                    relas = sec_data;
                    rela_num = shdr->sh_size / sizeof(struct upatch_relocation);
                } else if (strncmp(sec_name, TEXT_RELA_NAME, strlen(TEXT_RELA_NAME)) == 0) {
                    resolve_got_reloc_count(patch_file, sec_data, shdr->sh_size / shdr->sh_entsize);
                }
                break;

            default:
                break;
        }
    }

    if (unlikely(!patch_file->symtab_index || !patch_file->strtab_index)) {
        log_err("patch contains no symbol\n");
        ret = -ENOEXEC;
        goto out;
    }
    if (unlikely(!patch_file->func_index || !patch_file->funcs || !patch_file->func_num)) {
        log_err("patch contains no function\n");
        ret = -ENOEXEC;
        goto out;
    }
    if (unlikely(!patch_file->rela_index || !patch_file->string_index ||
        !relas || !patch_file->strings ||
        !rela_num || !patch_file->string_len ||
        patch_file->func_num != rela_num)) {
        log_err("invalid patch format\n");
        ret = -ENOEXEC;
        goto out;
    }

    for (i = 0; i < rela_num; i++) {
        patch_file->funcs[i].name_off = relas[i].name.r_addend;
    }

out:
    if (unlikely(ret)) {
        destroy_patch_file(patch_file);
    }

    return ret;
}

static int resolve_patch(struct patch_entity *patch, struct file *file)
{
    int ret;

    ret = resolve_patch_file(&patch->file, file);
    if (ret) {
        return ret;
    }
    patch->status = UPATCH_STATUS_NOT_APPLIED;

    INIT_HLIST_NODE(&patch->node);
    INIT_LIST_HEAD(&patch->actived_node);
    kref_init(&patch->kref);

    return 0;
}

static void destroy_patch(struct patch_entity *patch)
{
    WARN_ON(!hlist_unhashed(&patch->node));
    WARN_ON(!list_empty(&patch->actived_node));

    destroy_patch_file(&patch->file);
    patch->status = UPATCH_STATUS_NOT_APPLIED;

    INIT_HLIST_NODE(&patch->node);
    INIT_LIST_HEAD(&patch->actived_node);
}

/* --- Public interface --- */

struct patch_entity *load_patch(struct file *file)
{
    struct patch_entity *patch = NULL;
    int ret;

    if (unlikely(!file)) {
        return ERR_PTR(-EINVAL);
    }

    patch = kzalloc(sizeof(struct patch_entity), GFP_KERNEL);
    if (unlikely(!patch)) {
        return ERR_PTR(-ENOMEM);
    }

    ret = resolve_patch(patch, file);
    if (unlikely(ret)) {
        kfree(patch);
        return ERR_PTR(ret);
    }

    log_debug("new patch %s\n", patch->file.path);
    return patch;
}

void release_patch(struct kref *kref)
{
    struct patch_entity *patch;

    if (unlikely(!kref)) {
        return;
    }

    patch = container_of(kref, struct patch_entity, kref);
    log_debug("free patch %s\n", patch->file.path);

    destroy_patch(patch);
    kfree(patch);
}
