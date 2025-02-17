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

#include "running-elf.h"
#include "log.h"

/* TODO:
 * need to judge whether running_elf is a Position-Independent Executable file
 * https://github.com/bminor/binutils-gdb/blob/master/binutils/readelf.c
 */
static bool is_pie(void)
{
    return true;
}

static bool is_exec(struct Elf *elf)
{
    GElf_Ehdr ehdr;

    if (!gelf_getehdr(elf, &ehdr)) {
        ERROR("gelf_getehdr running_file failed for %s.", elf_errmsg(0));
    }
    return ehdr.e_type == ET_EXEC || (ehdr.e_type == ET_DYN && is_pie());
}

void relf_init(char *elf_name, struct running_elf *relf)
{
    GElf_Shdr shdr;
    Elf_Scn *scn = NULL;
    Elf_Data *data;
    GElf_Sym sym;

    relf->fd = open(elf_name, O_RDONLY);
    if (relf->fd == -1) {
        ERROR("open with errno = %d", errno);
    }

    relf->elf = elf_begin(relf->fd, ELF_C_READ, NULL);
    if (!relf->elf) {
        ERROR("elf_begin with error %s", elf_errmsg(0));
    }

    relf->is_exec = is_exec(relf->elf);

    while ((scn = elf_nextscn(relf->elf, scn)) != NULL) {
        if (!gelf_getshdr(scn, &shdr)) {
            ERROR("gelf_getshdr with error %s", elf_errmsg(0));
        }
        if (shdr.sh_type == SHT_SYMTAB) {
            break;
        }
    }

    data = elf_getdata(scn, NULL);
    if (!data) {
        ERROR("elf_getdata with error %s", elf_errmsg(0));
    }
    relf->obj_nr = (int)(shdr.sh_size / shdr.sh_entsize);
    relf->obj_syms = calloc((size_t)relf->obj_nr, sizeof(struct debug_symbol));
    if (!relf->obj_syms) {
        ERROR("calloc with errno = %d", errno);
    }

    for (int i = 0; i < relf->obj_nr; i++) {
        if (!gelf_getsym(data, i, &sym)) {
            ERROR("gelf_getsym with error %s", elf_errmsg(0));
        }
        relf->obj_syms[i].name = elf_strptr(relf->elf,
            shdr.sh_link, sym.st_name);
        if (!relf->obj_syms[i].name) {
            ERROR("elf_strptr with error %s", elf_errmsg(0));
        }
        relf->obj_syms[i].type = GELF_ST_TYPE(sym.st_info);
        relf->obj_syms[i].bind = GELF_ST_BIND(sym.st_info);
        relf->obj_syms[i].shndx = sym.st_shndx;
        relf->obj_syms[i].addr = sym.st_value;
        relf->obj_syms[i].size = sym.st_size;
    }
}

int relf_close(struct running_elf *relf)
{
    free(relf->obj_syms);
    elf_end(relf->elf);
    relf->elf = NULL;
    close(relf->fd);
    relf->fd = -1;

    return 0;
}

bool lookup_relf(struct running_elf *relf, struct symbol *lookup_sym,
    struct lookup_result *result)
{
    struct debug_symbol *symbol = NULL;
    unsigned long sympos = 0;

    log_debug("looking up symbol '%s'\n", lookup_sym->name);
    memset(result, 0, sizeof(*result));

    for (int i = 0; i < relf->obj_nr; i++) {
        symbol = &relf->obj_syms[i];
        sympos++;

        if (result->symbol != NULL && symbol->type == STT_FILE) {
            break;
        }
        if (strcmp(symbol->name, lookup_sym->name) != 0 ||
            symbol->bind != lookup_sym->bind) {
            continue;
        }

        if ((result->symbol != NULL) &&
            (result->symbol->bind == symbol->bind)) {
            ERROR("Found duplicate symbol '%s' in %s",
                lookup_sym->name, g_relf_name);
        }

        result->symbol = symbol;
        result->sympos = sympos;
        result->global =
            ((symbol->bind == STB_GLOBAL) || (symbol->bind == STB_WEAK));
        log_debug("found symbol '%s'\n", lookup_sym->name);
    }

    return (result->symbol != NULL);
}
