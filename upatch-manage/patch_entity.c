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

static const char *UPATCH_STRINGS_NAME = ".upatch.strings";
static const char *UPATCH_FUNCS_NAME = ".upatch.funcs";
static const char *RELA_TEXT_NAME = ".rela.text.";
static const char *REL_TEXT_NAME = ".rel.text.";

DEFINE_HASHTABLE(g_patches, PATCHES_HASH_BITS);
DEFINE_MUTEX(g_patch_table_lock);

static void free_patch_meta(struct upatch_metadata *meta)
{
    VFREE_CLEAR(meta->patch_buff);
    meta->func_count = 0;
    meta->patch_size = 0;
}

int search_got_rela_entry(struct file *patch, struct upatch_metadata *meta, Elf_Shdr *shdr)
{
    int type;
    int ret = 0;
    unsigned int i;
    Elf_Rela *rel = vmalloc_read(patch, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(rel)) {
        ret = PTR_ERR(rel);
        log_err("failed to read section '%s'\n", RELA_TEXT_NAME);
        return ret;
    }

    for (i = 0; i < shdr->sh_size / sizeof(*rel); i++) {
        type = ELF_R_TYPE(rel[i].r_info);
        if (is_got_rela_type(type)) {
            meta->got_rela_cnt++;
        }
    }

    VFREE_CLEAR(rel);
    return ret;
}

static int init_patch_meta(struct upatch_metadata *meta, struct file *patch)
{
    int ret = 0;

    Elf_Ehdr *ehdr;  // elf header
    Elf_Shdr *shdrs; // section headers
    char *shstrtab;  // .shstrtab

    Elf_Shdr *shdr;
    char *sh_name;
    void *sh_data;
    int sh_idx;
    void *hdr;
    size_t i;
    Elf_Sym *symtab;
    unsigned int symnum;

    meta->patch_size = i_size_read(file_inode(patch));
    hdr = vmalloc_read(patch, 0, meta->patch_size);
    if (IS_ERR(hdr)) {
        ret = PTR_ERR(hdr);
        log_err("read patch file for entity failed. ret=%d\n", ret);
        return ret;
    }
    meta->patch_buff = hdr;

    // elf header
    ehdr = hdr;

    if (!is_elf_valid(ehdr, meta->patch_size, true)) {
        ret = -EINVAL;
        log_err("invalid patch format\n");
        goto fail;
    }

    // section headers
    shdrs = hdr + ehdr->e_shoff;

    // section header string table
    shdr = &shdrs[ehdr->e_shstrndx];
    shstrtab = hdr + shdr->sh_offset;

    // resolve section headers
    for (sh_idx = 1; sh_idx < ehdr->e_shnum; sh_idx++) {
        shdr = &shdrs[sh_idx];
        sh_name = shstrtab + shdr->sh_name;
        sh_data = hdr + shdr->sh_offset;

        if (shdr->sh_type == SHT_SYMTAB) {
            meta->index.sym = sh_idx;
            meta->index.str = shdrs[sh_idx].sh_link;
            symtab = sh_data;
            symnum = shdr->sh_size / sizeof(Elf_Sym);
        } else if (strcmp(sh_name, UPATCH_STRINGS_NAME) == 0) {
            meta->strings = sh_data;
        } else if (strcmp(sh_name, UPATCH_FUNCS_NAME) == 0) {
            shdr->sh_entsize = sizeof(struct upatch_function);
            meta->func_count = shdr->sh_size / shdr->sh_entsize;
            meta->funcs = sh_data;
        } else if ((shdr->sh_type == SHT_RELA && !strncmp(sh_name, RELA_TEXT_NAME, strlen(RELA_TEXT_NAME))) ||
                (shdr->sh_type == SHT_REL && !strncmp(sh_name, REL_TEXT_NAME, strlen(REL_TEXT_NAME)))) {
            if (search_got_rela_entry(patch, meta, shdr)) {
                goto fail;
            }
        }
    }

    if (!meta->index.sym) {
        log_err("patch has no symbols (stripped?)\n");
        ret = -EINVAL;
        goto fail;
    }

    if (meta->func_count == 0) {
        log_err("patch has no .upatch.funcs\n");
        ret = -EINVAL;
        goto fail;
    }

    // search UND symbol number
    for (i = 1; i < symnum; i++) {
        if (symtab[i].st_shndx == SHN_UNDEF)
            meta->und_count++;
    }

    return 0;

fail:
    free_patch_meta(meta);
    return ret;
}

static int init_grab_patch(struct patch_entity *patch, const char *file_path)
{
    int ret = 0;
    struct file *file = NULL;

    INIT_HLIST_NODE(&patch->node);
    INIT_LIST_HEAD(&patch->patch_node);
    INIT_LIST_HEAD(&patch->actived_node);

    // open patch file
    file = filp_open(file_path, O_RDONLY, 0);
    if (IS_ERR(file)) {
        log_err("failed to open file '%s'\n", file_path);
        return PTR_ERR(file);
    }

    patch->inode = igrab(file_inode(file));
    if (!patch->inode) {
        pr_err("%s: failed to grab inode of '%s'\n", __func__, file_path);
        ret = -ENOENT;
        goto fail;
    }

    patch->path = kstrdup(file_path, GFP_KERNEL);
    if (!patch->path) {
        ret = -ENOMEM;
        iput(patch->inode);
        goto fail;
    }

    // resolve patch metadata
    ret = init_patch_meta(&patch->meta, file);
    if (ret != 0) {
        iput(patch->inode);
        KFREE_CLEAR(patch->path);
        log_err("failed to resolve patch meta, ret=%d\n", ret);
        goto fail;
    }

    patch->status = UPATCH_STATUS_DEACTIVED;

fail:
    filp_close(file, NULL);
    return ret;
}

struct patch_entity *get_patch_entity_from_inode(struct inode *inode)
{
    struct patch_entity *patch;
    struct patch_entity *found = NULL;

    mutex_lock(&g_patch_table_lock);
    hash_for_each_possible(g_patches, patch, node, inode->i_ino) {
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
    struct inode *inode;
    struct patch_entity *patch;

    inode = path_inode(path);
    if (IS_ERR(inode)) {
        return NULL;
    }

    inode = igrab(inode);
    if (!inode) {
        pr_err("%s: Failed to grab inode of %s\n", __func__, path);
        return NULL;
    }

    patch = get_patch_entity_from_inode(inode);
    iput(inode);

    return patch;
}

static inline void insert_patch(struct patch_entity *patch)
{
    mutex_lock(&g_patch_table_lock);
    hash_add(g_patches, &patch->node, patch->inode->i_ino);
    mutex_unlock(&g_patch_table_lock);
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

    ret = init_grab_patch(patch, file_path);
    if (ret) {
        log_err("failed to init patch '%s', ret=%d\n", file_path, ret);
        kfree(patch);
        return ERR_PTR(ret);
    }

    insert_patch(patch);
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
    free_patch_meta(&patch->meta);

    list_del(&patch->actived_node);
    list_del(&patch->patch_node);
    hash_del(&patch->node);

    kfree(patch);
}

void __exit verify_patch_empty_on_exit(void)
{
    struct patch_entity *patch;
    int bkt;

    mutex_lock(&g_patch_table_lock);
    hash_for_each(g_patches, bkt, patch, node) {
        log_err("found patch '%s' (%s) on exit",
            patch->path ? patch->path : "(null)", patch_status(patch->status));
    }
    mutex_unlock(&g_patch_table_lock);
}
