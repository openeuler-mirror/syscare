// SPDX-License-Identifier: GPL-2.0
/*
 * running-elf.c
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
#include <gelf.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>

#include <sys/types.h>
#include <sys/stat.h>

#include "upatch-elf.h"
#include "running-elf.h"
#include "log.h"

void relf_open(struct running_elf *relf, const char *name)
{
    GElf_Ehdr ehdr;
    GElf_Shdr shdr;
    GElf_Sym sym;

    if (relf == NULL) {
        return;
    }

    relf->fd = open(name, O_RDONLY);
    if (relf->fd == -1) {
        ERROR("Failed to open '%s', %s", name, strerror(errno));
    }

    relf->elf = elf_begin(relf->fd, ELF_C_READ, NULL);
    if (!relf->elf) {
        ERROR("Failed to read file '%s', %s", name, elf_errmsg(0));
    }

    if (!gelf_getehdr(relf->elf, &ehdr)) {
        ERROR("Failed to read file '%s' elf header, %s", name, elf_errmsg(0));
    }
    relf->is_exec = ((ehdr.e_type == ET_EXEC) || (ehdr.e_type == ET_DYN));

    Elf_Scn *scn = NULL;
    while ((scn = elf_nextscn(relf->elf, scn)) != NULL) {
        if (!gelf_getshdr(scn, &shdr)) {
            ERROR("Failed to read file '%s' section header, %s",
                name, elf_errmsg(0));
        }
        if (shdr.sh_type == SHT_SYMTAB) {
            break;
        }
    }

    Elf_Data *data = elf_getdata(scn, NULL);
    if (!data) {
        ERROR("Failed to read file '%s' section data, %s", name, elf_errmsg(0));
    }
    relf->symbol_count = (GElf_Word)(shdr.sh_size / shdr.sh_entsize);
    relf->symbols = calloc((size_t)relf->symbol_count, sizeof(struct relf_symbol));
    if (!relf->symbols) {
        ERROR("Failed to alloc memory, %s", strerror(errno));
    }

    for (GElf_Word i = 0; i < relf->symbol_count; i++) {
        if (!gelf_getsym(data, (int)i, &sym)) {
            ERROR("Failed to read file '%s' symbol, index=%d, %s",
                name, i, elf_errmsg(0));
        }
        relf->symbols[i].name = elf_strptr(relf->elf,
            shdr.sh_link, sym.st_name);
        if (!relf->symbols[i].name) {
            ERROR("Failed to read file '%s' symbol name, index=%d, %s",
                name, i, elf_errmsg(0));
        }
        relf->symbols[i].index = i;
        relf->symbols[i].type = GELF_ST_TYPE(sym.st_info);
        relf->symbols[i].bind = GELF_ST_BIND(sym.st_info);
        relf->symbols[i].shndx = sym.st_shndx;
        relf->symbols[i].addr = sym.st_value;
        relf->symbols[i].size = sym.st_size;
    }
}

void relf_close(struct running_elf *relf)
{
    if (relf == NULL) {
        return;
    }
    if (relf->symbols) {
        free(relf->symbols);
    }
    if (relf->elf) {
        elf_end(relf->elf);
    }
    if (relf->fd > 0) {
        close(relf->fd);
    }
    relf->elf = NULL;
    relf->fd = -1;
}

struct relf_symbol* lookup_relf(struct running_elf *relf, struct symbol *sym)
{
    struct relf_symbol *result = NULL;

    for (GElf_Word i = 0; i < relf->symbol_count; i++) {
        struct relf_symbol *symbol = &relf->symbols[i];

        if ((result != NULL) && (symbol->type == STT_FILE)) {
            break;
        }
        if ((strcmp(symbol->name, sym->name) != 0) ||
            (symbol->bind != sym->bind)) {
            continue;
        }
        if ((result != NULL) && (result->bind == symbol->bind)) {
            ERROR("Found duplicate symbol '%s' in %s", sym->name, g_relf_name);
        }

        result = symbol;
    }

    return result;
}
