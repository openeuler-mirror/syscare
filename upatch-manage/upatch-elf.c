// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
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

#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "log.h"
#include "upatch-common.h"
#include "upatch-elf.h"
#include "upatch-ptrace.h"

static int read_from_offset(int fd, void **buf, unsigned long len, off_t offset)
{
    *buf = malloc(len);
    if (*buf == NULL) {
        return -errno;
    }

    ssize_t size = pread(fd, *buf, len, offset);
    if (size == -1) {
        return -errno;
    }

    return 0;
}

static int open_elf(struct elf_info *einfo, const char *name)
{
    int ret = 0;
    int fd = -1;
    struct stat st;

    fd = open(name, O_RDONLY);
    if (fd == -1) {
        ret = -errno;
        log_error("Failed to open file '%s'\n", name);
        goto out;
    }

    ret = stat(name, &st);
    if (ret != 0) {
        ret = -errno;
        log_error("Failed to stat file '%s'\n", name);
        goto out;
    }

    ret = read_from_offset(fd, (void **)&einfo->patch_buff,
        (unsigned long)st.st_size, 0);
    if (ret != 0) {
        log_error("Failed to read file '%s'\n", name);
        goto out;
    }

    einfo->name = name;
    einfo->inode = st.st_ino;
    einfo->patch_size = (unsigned long)st.st_size;
    einfo->hdr = (void *)einfo->patch_buff;
    einfo->shdrs = (void *)einfo->hdr + einfo->hdr->e_shoff;
    einfo->shstrtab = (void *)einfo->hdr +
        einfo->shdrs[einfo->hdr->e_shstrndx].sh_offset;

    void *einfo_eof = einfo->hdr + einfo->patch_size;
    if ((void *)einfo->shdrs > einfo_eof ||
        (void *)einfo->shstrtab > einfo_eof) {
        log_error("File '%s' is not a valid elf\n", name);
        ret = -ENOEXEC;
        goto out;
    }

    ret = 0;

out:
    if (fd > 0) {
        close(fd);
    }
    return ret;
}

int upatch_init(struct upatch_elf *uelf, const char *name)
{
    int ret = open_elf(&uelf->info, name);
    if (ret) {
        log_error("Failed to open file '%s'\n", name);
        return ret;
    }

    for (unsigned int i = 1; i < uelf->info.hdr->e_shnum; ++i) {
        char *sec_name = uelf->info.shstrtab + uelf->info.shdrs[i].sh_name;
        if (uelf->info.shdrs[i].sh_type == SHT_SYMTAB) {
            uelf->num_syms = uelf->info.shdrs[i].sh_size / sizeof(GElf_Sym);
            uelf->index.sym = i;
            uelf->index.str = uelf->info.shdrs[i].sh_link;
            uelf->strtab = (char *)uelf->info.hdr +
                uelf->info.shdrs[uelf->info.shdrs[i].sh_link].sh_offset;
        } else if (streql(sec_name, UPATCH_FUNC_NAME)) {
            uelf->index.upatch_funcs = i;
        } else if (streql(sec_name, UPATCH_FUNC_STRING)) {
            uelf->index.upatch_string = i;
        }
    }

    return 0;
}

static bool is_pie_elf(struct running_elf *relf)
{
    GElf_Shdr *shdr = &relf->info.shdrs[relf->index.dynamic];
    GElf_Dyn *dyns = (void *)relf->info.hdr + shdr->sh_offset;

    if (relf->index.dynamic == 0) {
        return false;
    }

    for (Elf64_Xword i = 0; i < shdr->sh_size / sizeof(GElf_Dyn); i++) {
        log_debug("Syminfo %lx, %lx\n", dyns[i].d_tag, dyns[i].d_un.d_val);
        if (dyns[i].d_tag == DT_FLAGS_1) {
            if ((dyns[i].d_un.d_val & DF_1_PIE) != 0) {
                return true;
            }
            break;
        }
    }

    return false;
}

static inline bool is_dyn_elf(struct running_elf *relf)
{
    return relf->info.hdr->e_type == ET_DYN;
}

int binary_init(struct running_elf *relf, const char *name)
{
    int ret = open_elf(&relf->info, name);
    if (ret) {
        log_error("Failed to open file '%s'\n", name);
        return ret;
    }

    for (unsigned int i = 1; i < relf->info.hdr->e_shnum; i++) {
        char *sec_name = relf->info.shstrtab + relf->info.shdrs[i].sh_name;
        if (relf->info.shdrs[i].sh_type == SHT_SYMTAB) {
            log_debug("Found section '%s', idx=%d\n", SYMTAB_NAME, i);
            relf->num_syms = relf->info.shdrs[i].sh_size / sizeof(GElf_Sym);
            relf->index.sym = i;
            relf->index.str = relf->info.shdrs[i].sh_link;
            relf->strtab = (char *)relf->info.hdr +
                relf->info.shdrs[relf->info.shdrs[i].sh_link].sh_offset;
        } else if (relf->info.shdrs[i].sh_type == SHT_DYNSYM) {
            log_debug("Found section '%s', idx=%d\n", DYNSYM_NAME, i);
            relf->index.dynsym = i;
            relf->index.dynstr = relf->info.shdrs[i].sh_link;
            relf->dynstrtab = (char *)relf->info.hdr +
                relf->info.shdrs[relf->info.shdrs[i].sh_link].sh_offset;
        } else if (relf->info.shdrs[i].sh_type == SHT_DYNAMIC) {
        log_debug("Found section '%s', idx=%d\n", DYNAMIC_NAME, i);
            relf->index.dynamic = i;
        } else if (streql(sec_name, PLT_RELA_NAME) &&
            relf->info.shdrs[i].sh_type == SHT_RELA) {
            log_debug("Found section '%s', idx=%d\n", PLT_RELA_NAME, i);
            relf->index.rela_plt = i;
        } else if (streql(sec_name, GOT_RELA_NAME) &&
            relf->info.shdrs[i].sh_type == SHT_RELA) {
            log_debug("Found section '%s' idx=%d\n", GOT_RELA_NAME, i);
            relf->index.rela_dyn = i;
        }
    }

    relf->phdrs = (void *)relf->info.hdr + relf->info.hdr->e_phoff;
    for (int i = 0; i < relf->info.hdr->e_phnum; i++) {
        if (relf->phdrs[i].p_type == PT_TLS) {
            relf->tls_size = relf->phdrs[i].p_memsz;
            relf->tls_align = relf->phdrs[i].p_align;
            log_debug("Found TLS size = %ld, align = %ld\n",
                relf->tls_size, relf->tls_align);
            break;
        }
    }

    relf->info.is_pie = is_pie_elf(relf);
    relf->info.is_dyn = is_dyn_elf(relf);

    return 0;
}

void binary_close(struct running_elf *relf)
{
    if (relf->info.patch_buff) {
        free(relf->info.patch_buff);
    }
}

void upatch_close(struct upatch_elf *uelf)
{
    if (uelf->info.patch_buff) {
        free(uelf->info.patch_buff);
    }
    if (uelf->core_layout.kbase) {
        free(uelf->core_layout.kbase);
    }
}

bool is_upatch_section(const char *name)
{
    return !strncmp(name, ".upatch.", strlen(".upatch."));
}

bool is_note_section(GElf_Word type)
{
    return type == SHT_NOTE;
}
