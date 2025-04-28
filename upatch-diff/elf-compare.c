// SPDX-License-Identifier: GPL-2.0
/*
 * elf-compare.c
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

#include <libgen.h>

#include "log.h"
#include "elf-common.h"
#include "elf-compare.h"
#include "elf-insn.h"

static int compare_correlated_section(struct section *sec, struct section *twin);

static void compare_correlated_symbol(struct symbol *sym, struct symbol *twin)
{
    // symbol type & binding cannot be changed
    if (sym->type != twin->type) {
        ERROR("Symbol '%s' type mismatched", sym->name);
    }
    if (sym->sym.st_info != twin->sym.st_info) {
        ERROR("Symbol '%s' st_info mismatched", sym->name);
    }
    // object symbol size cannot be changed
    if ((sym->type == STT_OBJECT) && (sym->sym.st_size != twin->sym.st_size)) {
        ERROR("Symbol '%s' object size mismatched", sym->name);
    }

    /*
     * For local symbols, we handle them based on their matching sections.
     */
    if ((sym->sym.st_shndx == SHN_UNDEF) || (sym->sym.st_shndx == SHN_ABS)) {
        sym->status = SAME;
        return;
    }

    if ((sym->sec == NULL) || (sym->sym.st_shndx == SHN_ABS)) {
        ERROR("Symbol '%s' don't have section\n", sym->name);
    }

    if (sym->sec->twin != twin->sec) {
        ERROR("Symbol '%s' section mismatched", sym->name);
    }

    compare_correlated_section(sym->sec, twin->sec);
    if (sym->sec->status == CHANGED) {
        sym->status = CHANGED;
    } else if (!is_rela_section(sym->sec) &&
        (sym->sec->rela != NULL) &&
        (sym->sec->rela->status == CHANGED)) {
        sym->status = CHANGED;
    } else {
        sym->status = SAME;
    }
}

void upatch_compare_symbols(struct upatch_elf *uelf)
{
    struct symbol *sym;

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (is_symbol_ignored(sym)) {
            continue;
        }
        if (sym->twin) {
            compare_correlated_symbol(sym, sym->twin);
        } else {
            sym->status = NEW;
        }
        log_debug("symbol %s is %s\n", sym->name, status_str(sym->status));
    }
}

static bool rela_equal(struct rela *rela1, struct rela *rela2)
{
    if (rela1->type != rela2->type || rela1->offset != rela2->offset) {
        return false;
    }
    /* TODO: handle altinstr_aux */
    /* TODO: handle rela for toc section */
    if (rela1->string) {
        return rela2->string && !strcmp(rela1->string, rela2->string);
    }
    if (rela1->addend != rela2->addend) {
        log_debug("relocation addend has changed from %ld to %ld",
            rela1->addend, rela2->addend);
        return false;
    }

    return !mangled_strcmp(rela1->sym->name, rela2->sym->name);
}

static void compare_correlated_rela_section(struct section *relasec,
    struct section *relasec_twin)
{
    struct rela *rela1 = NULL;
    struct rela *rela2 = NULL;

    /* check relocation item one by one, order matters */
    rela2 = list_entry(relasec_twin->relas.next, struct rela, list);
    list_for_each_entry(rela1, &relasec->relas, list) {
        if (rela_equal(rela1, rela2)) {
            rela2 = list_entry(rela2->list.next, struct rela, list);
            continue;
        }
        relasec->status = CHANGED;
        return;
    }
    relasec->status = SAME;
}

static void compare_correlated_nonrela_section(struct section *sec,
    struct section *sectwin)
{
    if (sec->sh.sh_type != SHT_NOBITS &&
        (sec->data->d_size != sectwin->data->d_size ||
        memcmp(sec->data->d_buf, sectwin->data->d_buf, sec->data->d_size))) {
        sec->status = CHANGED;
    } else {
        sec->status = SAME;
    }
}

// we may change status of sec, they are not same
static int compare_correlated_section(struct section *sec, struct section *twin)
{
    /* We allow sh_flags and sh_addralign changes.
       When we change the initial value of variables
       sh_flags & sh_addralign may change in .rodata section */
    if ((sec->sh.sh_type != twin->sh.sh_type) ||
        (sec->sh.sh_entsize != twin->sh.sh_entsize)) {
        ERROR("%s section header details differ from %s",
            sec->name, twin->name);
        return -1;
    }
    if (sec->sh.sh_flags != twin->sh.sh_flags) {
        log_warn("Section '%s' sh_flags changed from %ld to %ld\n",
            sec->name, sec->sh.sh_flags, twin->sh.sh_flags);
    }
    if (sec->sh.sh_addralign != twin->sh.sh_addralign) {
        log_warn("Section '%s' sh_addralign changed from %ld to %ld\n",
            sec->name, sec->sh.sh_addralign, twin->sh.sh_addralign);
    }

    if (is_note_section(sec)) {
        sec->status = SAME;
        goto out;
    }
    /* As above but for aarch64 */
    if (!strcmp(sec->name, ".rela__patchable_function_entries") ||
        !strcmp(sec->name, "__patchable_function_entries")) {
        sec->status = SAME;
        goto out;
    }
    /* compare file size and data size(memory size) */
    if (sec->sh.sh_size != twin->sh.sh_size ||
        sec->data->d_size != twin->data->d_size ||
        (sec->rela && !twin->rela) || (!sec->rela && twin->rela)) {
        sec->status = CHANGED;
        goto out;
    }

    if (is_rela_section(sec)) {
        compare_correlated_rela_section(sec, twin);
    } else {
        compare_correlated_nonrela_section(sec, twin);
    }

out:
    if (sec->status == CHANGED) {
        log_debug("section %s has changed\n", sec->name);
    }

    return 0;
}

static void update_section_status(struct section *sec, enum status status)
{
    if (sec == NULL) {
        return;
    }
    if (sec->twin != NULL) {
        sec->twin->status = status;
    }
    if (is_rela_section(sec)) {
        if ((sec->base != NULL) && (sec->base->sym != NULL) && status != SAME) {
            sec->base->sym->status = status;
        }
    } else {
        if (sec->sym != NULL) {
            sec->sym->status = status;
        }
    }
}

void upatch_compare_sections(struct upatch_elf *uelf)
{
    struct section *sec = NULL;

    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->ignored) {
            continue;
        }
        if (sec->twin == NULL) {
            sec->status = NEW;
        } else {
            compare_correlated_section(sec, sec->twin);
        }
        /* sync status */
        update_section_status(sec, sec->status);
        update_section_status(sec->twin, sec->status);
    }
}
