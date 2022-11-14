/*
 * elf-debug.c
 *
 * Copyright (C) 2014 Seth Jennings <sjenning@redhat.com>
 * Copyright (C) 2013-2014 Josh Poimboeuf <jpoimboe@redhat.com>
 * Copyright (C) 2022 Longjun Luo <luolongjun@huawei.com>
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

#include <string.h>
#include <stdlib.h>

#include "log.h"
#include "list.h"
#include "elf-common.h"
#include "elf-debug.h"
#include "upatch-elf.h"

void upatch_print_changes(struct upatch_elf *uelf)
{
    struct symbol *sym;

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (!sym->include || !sym->sec || sym->type != STT_FUNC || sym->parent)
            continue;
        if (sym->status == NEW)
            log_normal("new function: %s\n", sym->name);
        else if (sym->status == CHANGED)
            log_normal("changed function: %s\n", sym->name);
    }
}

void upatch_dump_kelf(struct upatch_elf *uelf)
{
    struct section *sec;
    struct symbol *sym;
    struct rela *rela;

    log_debug("\n=== Sections ===\n");
    list_for_each_entry(sec, &uelf->sections, list) {
        log_debug("%02d %s (%s)", sec->index, sec->name, status_str(sec->status));
        if (is_rela_section(sec)) {
            log_debug(", base-> %s\n", sec->base->name);
            if (is_debug_section(sec))
                goto next;
            log_debug("rela section expansion\n");
            list_for_each_entry(rela, &sec->relas, list) {
                log_debug("sym %d, offset %d, type %d, %s %s %ld \n",
                    rela->sym->index, rela->offset,
                    rela->type, rela->sym->name,
                    (rela->addend < 0) ? "-" : "+",
                    labs(rela->addend));
            }
        } else {
            if (sec->sym)
                log_debug(", sym-> %s", sec->sym->name);
            if (sec->secsym)
                log_debug(", secsym-> %s", sec->secsym->name);
            if (sec->rela)
                log_debug(", rela-> %s", sec->rela->name);
        }
next:
        log_debug("\n");
    }

    log_debug("\n=== Symbols ===\n");
    list_for_each_entry(sym, &uelf->symbols, list) {
        log_debug("sym %02d, type %d, bind %d, ndx %02d, name %s (%s)",
            sym->index, sym->type, sym->bind, sym->sym.st_shndx,
            sym->name, status_str(sym->status));
        if (sym->sec && (sym->type == STT_FUNC || sym->type == STT_OBJECT))
            log_debug(" -> %s", sym->sec->name);
        log_debug("\n");
    }
}









