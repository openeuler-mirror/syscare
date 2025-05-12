// SPDX-License-Identifier: GPL-2.0
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

void upatch_print_correlation(struct upatch_elf *uelf)
{
    if (uelf == NULL) {
        return;
    }

    log_debug("\n------------------------------\n");
    log_debug("Section\n");
    log_debug("------------------------------\n");
    struct section *sec;
    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->twin != NULL) {
            log_debug("index: %04d, name: '%s' -> index: %04d, name: '%s'\n",
                sec->index, sec->name, sec->twin->index, sec->twin->name);
        } else {
            log_debug("index: %04d, name: '%s' -> None\n",
                sec->index, sec->name);
        }
    }
    log_debug("------------------------------\n");
    log_debug("\n");
    log_debug("------------------------------\n");
    log_debug("Symbol\n");
    log_debug("------------------------------\n");
    struct symbol *sym;
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->twin != NULL) {
            log_debug("index: %04d, name: '%s' -> index: %04d, name: '%s'\n",
                sym->index, sym->name, sym->twin->index, sym->twin->name);
        } else {
            log_debug("index: %04d, name: '%s' -> None\n",
                sym->index, sym->name);
        }
    }
    log_debug("------------------------------\n");
}

void upatch_print_changes(struct upatch_elf *uelf)
{
    struct symbol *sym;
    struct section *sec;

    if (uelf == NULL) {
        return;
    }

    log_normal("\n------------------------------\n");
    log_normal("New symbol\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->status == NEW) {
            log_normal("index: %04d, name: '%s'\n", sym->index, sym->name);
        }
    }
    log_normal("------------------------------\n");
    log_normal("\n");
    log_normal("------------------------------\n");
    log_normal("New section\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->status == NEW) {
            log_normal("index: %04d, name: '%s'\n", sec->index, sec->name);
        }
    }
    log_normal("------------------------------\n");
    log_normal("\n");
    log_normal("------------------------------\n");
    log_normal("Changed symbol\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->status == CHANGED) {
            log_normal("index: %04d, name: '%s'\n", sym->index, sym->name);
        }
    }
    log_normal("------------------------------\n");
    log_normal("\n");
    log_normal("------------------------------\n");
    log_normal("Changed section\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->status == CHANGED) {
            log_normal("index: %04d, name: '%s'\n", sec->index, sec->name);
        }
    }
    log_normal("------------------------------\n");
    log_normal("\n");
    log_normal("------------------------------\n");
    log_normal("Included symbol\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->include) {
            log_normal("index: %04d, name: '%s', status: %s\n",
                sym->index, sym->name, status_str(sym->status));
        }
    }
    log_normal("------------------------------\n");
    log_normal("\n");
    log_normal("------------------------------\n");
    log_normal("Included section\n");
    log_normal("------------------------------\n");
    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->include) {
            log_normal("index: %04d, name: '%s', status: %s\n",
                sec->index, sec->name, status_str(sec->status));
        }
    }
    log_normal("------------------------------\n");
}
