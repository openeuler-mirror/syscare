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
#include <string.h>

#include "log.h"
#include "upatch-common.h"
#include "upatch-elf.h"
#include "upatch-resolve.h"

static unsigned long resolve_rela_dyn(struct upatch_elf *uelf,
    struct object_file *obj, const char *name, GElf_Sym *patch_sym)
{
    unsigned long elf_addr = 0;
    struct running_elf *relf = uelf->relf;

    if (!relf || !relf->index.dynsym || !relf->index.rela_dyn) {
        return 0;
    }

    GElf_Shdr *dynsym_shdr = &relf->info.shdrs[relf->index.dynsym];
    GElf_Shdr *rela_dyn_shdr = &relf->info.shdrs[relf->index.rela_dyn];

    GElf_Sym *dynsym = (void *)relf->info.hdr + dynsym_shdr->sh_offset;
    GElf_Rela *rela_dyn = (void *)relf->info.hdr + rela_dyn_shdr->sh_offset;

    for (Elf64_Xword i = 0; i < rela_dyn_shdr->sh_size / sizeof(GElf_Rela); i++) {
        unsigned long sym_idx = GELF_R_SYM(rela_dyn[i].r_info);

        if (sym_idx == 0) {
            /*
             * some rela don't have the symbol index, use the symbol's value and
             * rela's addend to find the symbol. for example, R_X86_64_IRELATIVE.
             */
            if (rela_dyn[i].r_addend != (long)patch_sym->st_value) {
                continue;
            }
        }
        else {
            char *sym_name = relf->dynstrtab + dynsym[sym_idx].st_name;
            char *sym_splitter = NULL;

            /* strip symbol version if exists */
            sym_splitter = strchr(sym_name, '@');
            if (sym_splitter != NULL) {
                *sym_splitter = '\0';
            }

            /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
            if (!streql(sym_name, name)) {
                continue;
            }
        }

        /* r_offset is virtual address of GOT table */
        unsigned long sym_addr = relf->load_bias + rela_dyn[i].r_offset;
        elf_addr = insert_got_table(uelf, obj, GELF_R_TYPE(rela_dyn[i].r_info), sym_addr);

        log_debug("resolved %s from .rela_dyn at 0x%lx\n", name, elf_addr);
        break;
    }

    return elf_addr;
}

static unsigned long resolve_rela_plt(struct upatch_elf *uelf,
    struct object_file *obj, const char *name, GElf_Sym *patch_sym)
{
    unsigned long elf_addr = 0;
    struct running_elf *relf = uelf->relf;

    if (!relf || !relf->index.dynsym || !relf->index.rela_plt) {
        return 0;
    }

    GElf_Shdr *dynsym_shdr = &relf->info.shdrs[relf->index.dynsym];
    GElf_Shdr *rela_plt_shdr = &relf->info.shdrs[relf->index.rela_plt];

    GElf_Sym *dynsym = (void *)relf->info.hdr + dynsym_shdr->sh_offset;
    GElf_Rela *rela_plt = (void *)relf->info.hdr + rela_plt_shdr->sh_offset;

    for (Elf64_Xword i = 0; i < rela_plt_shdr->sh_size / sizeof(GElf_Rela); i++) {
        unsigned long sym_idx = GELF_R_SYM(rela_plt[i].r_info);
        unsigned long sym_type = GELF_ST_TYPE(dynsym[sym_idx].st_info);

        if (sym_type != STT_FUNC && sym_type != STT_TLS) {
            continue;
        }

        if (sym_idx == 0) {
            /*
             * some rela don't have the symbol index, use the symbol's value and
             * rela's addend to find the symbol. for example, R_X86_64_IRELATIVE.
             */
            if (rela_plt[i].r_addend != (long)patch_sym->st_value) {
                continue;
            }
        } else {
            char *sym_name = relf->dynstrtab + dynsym[sym_idx].st_name;
            char *sym_splitter = NULL;

            /* strip symbol version if exists */
            sym_splitter = strchr(sym_name, '@');
            if (sym_splitter != NULL) {
                *sym_splitter = '\0';
            }

            if (!streql(sym_name, name)) {
                continue;
            }
        }

        /* r_offset is virtual address of PLT table */
        unsigned long sym_addr = relf->load_bias + rela_plt[i].r_offset;
        elf_addr = insert_plt_table(uelf, obj, GELF_R_TYPE(rela_plt[i].r_info), sym_addr);

        log_debug("Resolved '%s' from '.rela_plt' at 0x%lx\n", name, elf_addr);
        break;
    }

    return elf_addr;
}

static unsigned long resolve_dynsym(struct upatch_elf *uelf,
    struct object_file *obj, const char *name)
{
    unsigned long elf_addr = 0;
    struct running_elf *relf = uelf->relf;

    if (!relf || !relf->index.dynsym) {
        return 0;
    }

    GElf_Shdr *dynsym_shdr = &relf->info.shdrs[relf->index.dynsym];
    GElf_Sym *dynsym = (void *)relf->info.hdr + dynsym_shdr->sh_offset;

    for (Elf64_Xword i = 0; i < dynsym_shdr->sh_size / sizeof(GElf_Sym); i++) {
        if (dynsym[i].st_value == 0) {
            continue;
        }

        char *sym_name = relf->dynstrtab + dynsym[i].st_name;
        char *sym_splitter = strchr(sym_name, '@');
        if (sym_splitter != NULL) {
            *sym_splitter = '\0';
        }

        /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
        if (!streql(sym_name, name)) {
            continue;
        }

        unsigned long sym_addr = relf->load_bias + dynsym[i].st_value;
        elf_addr = insert_got_table(uelf, obj, 0, sym_addr);

        log_debug("Resolved '%s' from '.dynsym' at 0x%lx\n", name, elf_addr);
        break;
    }

    return elf_addr;
}

static unsigned long resolve_sym(struct upatch_elf *uelf, const char *name)
{
    unsigned long elf_addr = 0;
    struct running_elf *relf = uelf->relf;

    if (!relf || !relf->index.sym) {
        return 0;
    }

    GElf_Shdr *sym_shdr = &relf->info.shdrs[relf->index.sym];
    GElf_Sym *sym = (void *)relf->info.hdr + sym_shdr->sh_offset;

    for (Elf64_Xword i = 0; i < sym_shdr->sh_size / sizeof(GElf_Sym); i++) {
        if (sym[i].st_shndx == SHN_UNDEF) {
            continue;
        }

        /* strip symbol version if exists */
        char *sym_name = relf->strtab + sym[i].st_name;
        char *sym_splitter = strchr(sym_name, '@');
        if (sym_splitter != NULL) {
            *sym_splitter = '\0';
        }

        if (!streql(sym_name, name)) {
            continue;
        }

        elf_addr = relf->load_bias + sym[i].st_value;

        log_debug("Resolved '%s' from '.sym' at 0x%lx\n", name, elf_addr);
        break;
    }

    return elf_addr;
}

static unsigned long resolve_patch_sym(struct upatch_elf *uelf,
    const char *name, GElf_Sym *patch_sym)
{
    unsigned long elf_addr = 0;
    struct running_elf *relf = uelf->relf;

    if (!relf) {
        return 0;
    }

    if (!patch_sym->st_value) {
        return 0;
    }

    elf_addr = relf->load_bias + patch_sym->st_value;
    log_debug("Resolved '%s' from patch '.sym' at 0x%lx\n", name, elf_addr);

    return elf_addr;
}

static unsigned long resolve_symbol(struct upatch_elf *uelf,
                    struct object_file *obj, const char *name,
                    GElf_Sym patch_sym)
{
    unsigned long elf_addr = 0;
    /*
     * Handle external symbol, several possible solutions here:
     * 1. use symbol address from .dynsym, but most of its address is still
     * undefined
     * 2. use address from PLT/GOT, problems are:
     *    1) range limit(use jmp table?)
     *    2) only support existed symbols
     * 3. read symbol from library, combined with load_bias, calculate it
     * directly and then worked with jmp table.
     *
     * Currently, we will try approach 1 and approach 2.
     * Approach 3 is more general, but difficulty to implement.
     */

	/* resolve from got */
    elf_addr = resolve_rela_dyn(uelf, obj, name, &patch_sym);

	/* resolve from plt */
    if (!elf_addr) {
        elf_addr = resolve_rela_plt(uelf, obj, name, &patch_sym);
    }

	/* resolve from dynsym */
    if (!elf_addr) {
        elf_addr = resolve_dynsym(uelf, obj, name);
    }

	/* resolve from sym */
    if (!elf_addr) {
        elf_addr = resolve_sym(uelf, name);
    }

	/* resolve from patch sym */
    if (!elf_addr) {
        elf_addr = resolve_patch_sym(uelf, name, &patch_sym);
    }

    if (!elf_addr) {
        log_error("Cannot resolve symbol '%s'\n", name);
    }
    return elf_addr;
}

int simplify_symbols(struct upatch_elf *uelf, struct object_file *obj)
{
    GElf_Sym *sym = (void *)uelf->info.shdrs[uelf->index.sym].sh_addr;
    unsigned long secbase;
    unsigned int i;
    int ret = 0;
    unsigned long elf_addr;

    for (i = 1; i < uelf->num_syms; i++) {
        const char *name;

        if (GELF_ST_TYPE(sym[i].st_info) == STT_SECTION &&
            sym[i].st_shndx < uelf->info.hdr->e_shnum)
            name = uelf->info.shstrtab + uelf->info.shdrs[sym[i].st_shndx].sh_name;
        else
            name = uelf->strtab + sym[i].st_name;

        switch (sym[i].st_shndx) {
        case SHN_COMMON:
            log_debug("Unsupported common symbol '%s'\n", name);
            ret = -ENOEXEC;
            break;
        case SHN_ABS:
            break;
        case SHN_UNDEF:
            elf_addr = resolve_symbol(uelf, obj, name, sym[i]);
            if (!elf_addr) {
                ret = -ENOEXEC;
            }
            sym[i].st_value = elf_addr;
            log_debug("Resolved symbol '%s' at 0x%lx\n",
                name, (unsigned long)sym[i].st_value);
            break;
        case SHN_LIVEPATCH:
            sym[i].st_value += uelf->relf->load_bias;
            log_debug("Resolved livepatch symbol '%s' at 0x%lx\n",
                  name, (unsigned long)sym[i].st_value);
            break;
        default:
            /* use real address to calculate secbase */
            secbase = uelf->info.shdrs[sym[i].st_shndx].sh_addralign;
            sym[i].st_value += secbase;
            log_debug("Symbol '%s' at 0x%lx\n",
                name, (unsigned long)sym[i].st_value);
            break;
        }
    }

    return ret;
}
