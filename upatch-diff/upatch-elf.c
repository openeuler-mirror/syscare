// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-elf.c
 *
 * Copyright (C) 2014 Seth Jennings <sjenning@redhat.com>
 * Copyright (C) 2013-2014 Josh Poimboeuf <jpoimboe@redhat.com>
 * Copyright (C) 2022 Longjun Luo <luolongjun@huawei.com>
 * Copyright (C) 2022 Zongwu Li <lizongwu@huawei.com>
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA,
 * 02110-1301, USA.
 */

#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <stdbool.h>

#include <gelf.h>

#include "elf-common.h"
#include "elf-insn.h"
#include "upatch-elf.h"
#include "list.h"
#include "log.h"

static void create_section_list(struct upatch_elf *uelf)
{
    size_t shstrndx = 0;
    if (elf_getshdrstrndx(uelf->elf, &shstrndx) != 0) {
        ERROR("Failed to get section header string index");
    }

    Elf_Scn *scn = elf_nextscn(uelf->elf, NULL);
    while (scn != NULL) {
        GElf_Section index = (GElf_Section)elf_ndxscn(scn);
        struct section *sec = NULL;

        ALLOC_LINK(sec, &uelf->sections);
        if (gelf_getshdr(scn, &sec->sh) == NULL) {
            ERROR("Failed to parse section, index=%d", index);
        }

        sec->index = (GElf_Section)index;
        sec->name = elf_strptr(uelf->elf, shstrndx, sec->sh.sh_name);
        if (sec->name == NULL) {
            ERROR("Failed to get section name, index=%d", index);
        }
        sec->data = elf_getdata(scn, NULL);
        if (sec->data == NULL) {
            ERROR("Failed to get section '%s' data, index=%d",
                sec->name, index);
        }

        sec->name_source = DATA_SOURCE_REF;
        sec->data_source = DATA_SOURCE_REF;
        sec->dbuf_source = DATA_SOURCE_REF;

        INIT_LIST_HEAD(&sec->relas);

        scn = elf_nextscn(uelf->elf, scn);
    }
}

static void create_symbol_list(struct upatch_elf *uelf)
{
    struct section *symtab = find_section_by_type(&uelf->sections, SHT_SYMTAB);
    if (symtab == NULL) {
        ERROR("Cannot find symbol table");
    }

    GElf_Word count = (GElf_Word)(symtab->sh.sh_size / symtab->sh.sh_entsize);
    for (GElf_Word i = 0; i < count; i++) {
        struct symbol *sym = NULL;

        ALLOC_LINK(sym, &uelf->symbols);
        if (gelf_getsym(symtab->data, (int)i, &sym->sym) == NULL) {
            ERROR("Failed to parse symbol, index=%d", i);
        }

        sym->index = i;
        sym->name = elf_strptr(uelf->elf, symtab->sh.sh_link, sym->sym.st_name);
        if (sym->name == NULL) {
            ERROR("Failed to get symbol name, index=%d", i);
        }

        sym->name_source = DATA_SOURCE_REF;

        sym->bind = GELF_ST_BIND(sym->sym.st_info);
        sym->type = GELF_ST_TYPE(sym->sym.st_info);
        GElf_Section shndx = sym->sym.st_shndx;
        if ((sym->sym.st_shndx > SHN_UNDEF) &&
            (sym->sym.st_shndx < SHN_LORESERVE)) {
            sym->sec = find_section_by_index(&uelf->sections, shndx);
            if (sym->sec == NULL) {
                ERROR("Failed to find symbol '%s' section, index=%d, shndx=%d",
                    sym->name, i, shndx);
            }
            if (sym->type == STT_SECTION) {
                /* sym must be the bundleable symbol */
                sym->sec->sym = sym;
                /* use section name as symbol name */
                sym->name = sym->sec->name;
            }
        }

        INIT_LIST_HEAD(&sym->children);
        INIT_LIST_HEAD(&sym->subfunction_node);
    }
}

static void create_rela_list(struct upatch_elf *uelf, struct section *sec)
{
    /* for relocation sections, sh_info is the index which these info apply */
    sec->base = (struct section *)sec->info;
    if (sec->base == NULL) {
        ERROR("Cannot find section '%s' base section, index=%d",
            sec->name, sec->index);
    }
    sec->base->rela = sec;

    GElf_Word count = (GElf_Word)(sec->sh.sh_size / sec->sh.sh_entsize);
    for (GElf_Word i = 0; i < count; i++) {
        struct rela *rela = NULL;

        ALLOC_LINK(rela, &sec->relas);
        if (gelf_getrela(sec->data, (int)i, &rela->rela) == NULL) {
            ERROR("Failed to parse rela, index=%d", i);
        }

        GElf_Word symndx = (GElf_Word)GELF_R_SYM(rela->rela.r_info);
        rela->sym = find_symbol_by_index(&uelf->symbols, symndx);
        if (rela->sym == NULL) {
            ERROR("Cannot find rela symbol, index=%d, symndx=%d", i, symndx);
        }
        rela->type = GELF_R_TYPE(rela->rela.r_info);
        rela->addend = rela->rela.r_addend;
        rela->offset = rela->rela.r_offset;

        if (is_string_section(rela->sym->sec)) {
            void *data = rela->sym->sec->data->d_buf;
            GElf_Addr addr = rela->sym->sym.st_value;
            long offset = rela_target_offset(uelf, sec, rela);

            rela->string = data + addr + offset;
            if (rela->string == NULL) {
                ERROR("Cannot find rela string %s+%ld",
                    rela->sym->name, rela->addend);
            }
        }
    }
}

static void destroy_rela_list(struct section *sec)
{
    struct rela *rela = NULL;
    struct rela *safe = NULL;

    list_for_each_entry_safe(rela, safe, &sec->relas, list) {
        list_del(&rela->list);
        free(rela);
    }

    INIT_LIST_HEAD(&sec->relas);
}

static void destroy_section_list(struct upatch_elf *uelf)
{
    struct section *sec = NULL;
    struct section *safesec = NULL;

    list_for_each_entry_safe(sec, safesec, &uelf->sections, list) {
        if (sec->twin) {
            sec->twin->twin = NULL;
        }

        if ((sec->name != NULL) && (sec->name_source == DATA_SOURCE_ALLOC)) {
            free(sec->name);
            sec->name = NULL;
        }

        if (sec->data != NULL) {
            if (sec->dbuf_source == DATA_SOURCE_ALLOC) {
                free(sec->data->d_buf);
                sec->data->d_buf = NULL;
            }
            if (sec->data_source == DATA_SOURCE_ALLOC) {
                free(sec->data);
                sec->data = NULL;
            }
        }

        if (is_rela_section(sec)) {
            destroy_rela_list(sec);
        }

        list_del(&sec->list);
        free(sec);
    }

    INIT_LIST_HEAD(&uelf->sections);
}

static void destroy_symbol_list(struct upatch_elf *uelf)
{
    struct symbol *sym = NULL;
    struct symbol *safesym = NULL;

    list_for_each_entry_safe(sym, safesym, &uelf->symbols, list) {
        if (sym->twin) {
            sym->twin->twin = NULL;
        }

        list_del(&sym->list);
        free(sym);
    }

    INIT_LIST_HEAD(&uelf->symbols);
}

static void destroy_string_list(struct upatch_elf *uelf)
{
    struct string *str = NULL;
    struct string *safestr = NULL;

    list_for_each_entry_safe(str, safestr, &uelf->strings, list) {
        list_del(&str->list);
        free(str);
    }

    INIT_LIST_HEAD(&uelf->strings);
}

static void parse_section_metadata(struct upatch_elf *uelf)
{
    struct section *sec;
    list_for_each_entry(sec, &uelf->sections, list) {
        /* find sh_link */
        if (sec->sh.sh_link != SHN_UNDEF) {
            sec->link = find_section_by_index(&uelf->sections,
                (GElf_Section)sec->sh.sh_link);
            if (sec->link == NULL) {
                ERROR("Cannot find '%s' link section, sh_link=%d",
                    sec->name, sec->sh.sh_link);
            }
        }
        /* find sh_info */
        if ((sec->sh.sh_type == SHT_REL) || (sec->sh.sh_type == SHT_RELA)) {
            sec->info = find_section_by_index(&uelf->sections,
                (GElf_Section)sec->sh.sh_info);
            if (sec->link == NULL) {
                ERROR("Cannot find '%s' info section, sh_info=%d",
                    sec->name, sec->sh.sh_link);
            }
        } else if (sec->sh.sh_type == SHT_GROUP) {
            sec->info = find_symbol_by_index(&uelf->symbols, sec->sh.sh_info);
            if (sec->link == NULL) {
                ERROR("Cannot find '%s' info symbol, sh_info=%d",
                    sec->name, sec->sh.sh_link);
            }
        }
        /* handle rela section */
        if (sec->sh.sh_type == SHT_RELA) {
            create_rela_list(uelf, sec);
        }
    }
}

void uelf_open(struct upatch_elf *uelf, const char *name)
{
    GElf_Ehdr ehdr;

    if (uelf == NULL) {
        return;
    }
    INIT_LIST_HEAD(&uelf->sections);
    INIT_LIST_HEAD(&uelf->symbols);
    INIT_LIST_HEAD(&uelf->strings);

    int fd = open(name, O_RDONLY);
    if (fd == -1) {
        ERROR("Failed to open '%s', %s", name, strerror(errno));
    }
    uelf->fd = fd;

    Elf *elf = elf_begin(fd, ELF_C_READ, NULL);
    if (!elf) {
        ERROR("Failed to read file '%s', %s", name, elf_errmsg(0));
    }
    uelf->elf = elf;

    if (!gelf_getehdr(uelf->elf, &ehdr)) {
        ERROR("Failed to read file '%s' elf header, %s", name, elf_errmsg(0));
    }
    if (ehdr.e_type != ET_REL) {
        ERROR("File '%s' is not object file", name);
    }

    /*
     * Main problem here is stack check, for kernel, only x86 is support
     * Not sure how to handle userspace, but let us handle x86 first here
     */
    switch (ehdr.e_machine) {
        case EM_AARCH64:
            uelf->arch = AARCH64;
            break;
        case EM_X86_64:
            uelf->arch = X86_64;
            break;
        default:
            ERROR("Unsupported architecture");
    }

    create_section_list(uelf);
    create_symbol_list(uelf);
    parse_section_metadata(uelf);
}

void uelf_close(struct upatch_elf *uelf)
{
    if (uelf == NULL) {
        return;
    }
    destroy_section_list(uelf);
    destroy_symbol_list(uelf);
    destroy_string_list(uelf);

    if (uelf->elf) {
        elf_end(uelf->elf);
    }
    if (uelf->fd > 0) {
        close(uelf->fd);
    }
    uelf->elf = NULL;
    uelf->fd = -1;
}
