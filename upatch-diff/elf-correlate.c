// SPDX-License-Identifier: GPL-2.0
/*
 * elf-correlate.c
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

#include "elf-common.h"
#include "elf-correlate.h"

static void correlate_symbol(struct symbol *sym_orig,
    struct symbol *sym_patched)
{
    sym_orig->twin = sym_patched;
    sym_patched->twin = sym_orig;
    sym_orig->status = sym_patched->status = SAME;
    if (strcmp(sym_orig->name, sym_patched->name)) {
        log_debug("renaming symbol %s to %s\n",
            sym_patched->name, sym_orig->name);
        sym_patched->name = sym_orig->name;
    }
}

void upatch_correlate_symbols(struct upatch_elf *uelf_source,
    struct upatch_elf *uelf_patched)
{
    struct symbol *sym_orig;
    struct symbol *sym_patched;

    list_for_each_entry(sym_orig, &uelf_source->symbols, list) {
        if (is_symbol_ignored(sym_orig)) {
            continue;
        }
        if (sym_orig->twin) {
            continue;
        }
        /*
         * Special static local variables should never be correlated
         * and should always be included if they are referenced by
         * an included function.
         */
        if (is_special_static_symbol(sym_orig)) {
            continue;
        }

        /* find matched symbol */
        list_for_each_entry(sym_patched, &uelf_patched->symbols, list) {
            if (mangled_strcmp(sym_orig->name, sym_patched->name) ||
                sym_orig->type != sym_patched->type || sym_patched->twin) {
                continue;
            }
            if (is_special_static_symbol(sym_patched)) {
                continue;
            }
            /*
             * The .LCx symbols point to string literals in
             * '.rodata.<func>.str1.*' sections.  They get included
             * in include_standard_elements().
             * Clang creates similar .Ltmp%d symbols in .rodata.str
             */
            if (sym_orig->type == STT_NOTYPE &&
                (!strncmp(sym_orig->name, ".LC", 3) ||
                    !strncmp(sym_orig->name, ".Ltmp", 5))) {
                continue;
            }

            if (is_mapping_symbol(uelf_source, sym_orig)) {
                continue;
            }

            /* group section symbols must have correlated sections */
            if (sym_orig->sec && sym_orig->sec->sh.sh_type == SHT_GROUP &&
                sym_orig->sec->twin != sym_patched->sec) {
                continue;
            }

            correlate_symbol(sym_orig, sym_patched);
            break;
        }
    }
}

static void correlate_section_impl(struct section *sec_orig,
    struct section *sec_patched)
{
    sec_orig->twin = sec_patched;
    sec_patched->twin = sec_orig;
    /* set initial status, might change */
    sec_orig->status = sec_patched->status = SAME;

    /* Make sure these two sections have the same name */
    if (strcmp(sec_orig->name, sec_patched->name)) {
        log_debug("renaming section %s to %s\n",
            sec_patched->name, sec_orig->name);
        sec_patched->name = sec_orig->name;
        sec_patched->name_source = DATA_SOURCE_REF;
    }
}

static void correlate_section(struct section *sec_orig,
    struct section *sec_patched)
{
    correlate_section_impl(sec_orig, sec_patched);

    if (is_rela_section(sec_orig)) {
        correlate_section_impl(sec_orig->base, sec_patched->base);
        /* handle symbol for base section now */
        sec_orig = sec_orig->base;
        sec_patched = sec_patched->base;
    } else if (sec_orig->rela && sec_patched->rela) {
        correlate_section_impl(sec_orig->rela, sec_patched->rela);
    }
    if (sec_orig->sym && sec_patched->sym) {
        correlate_symbol(sec_orig->sym, sec_patched->sym);
    }
    if (sec_orig->bundle_sym) {
        correlate_symbol(sec_orig->bundle_sym, sec_patched->bundle_sym);
    }
}

void upatch_correlate_sections(struct upatch_elf *uelf_source,
    struct upatch_elf *uelf_patched)
{
    struct section *sec_orig;
    struct section *sec_patched;

    list_for_each_entry(sec_orig, &uelf_source->sections, list) {
        /* already found */
        if (sec_orig->twin != NULL) {
            continue;
        }
        /*
         * Special static local variables should never be correlated
         * and should always be included if they are referenced by
         * an included function.
         */
        if (is_special_static_section(sec_orig)) {
            continue;
        }
        list_for_each_entry(sec_patched, &uelf_patched->sections, list) {
            if (sec_patched->twin != NULL) {
                continue;
            }
            if (is_special_static_section(sec_patched)) {
                continue;
            }
            if (mangled_strcmp(sec_orig->name, sec_patched->name)) {
                continue;
            }
            /*
             * Group sections must match exactly to be correlated.
             */
            if (sec_orig->sh.sh_type == SHT_GROUP) {
                if (sec_orig->data->d_size != sec_patched->data->d_size) {
                    continue;
                }
                if (memcmp(sec_orig->data->d_buf, sec_patched->data->d_buf,
                    sec_orig->data->d_size)) {
                    continue;
                }
            }

            correlate_section(sec_orig, sec_patched);
            break;
        }
    }
}

/* TODO: need handle .toc section */
static struct symbol *find_uncorrelated_rela(struct section *relasec,
    struct symbol *sym)
{
    struct rela *rela;

    /* find the patched object's corresponding variable */
    list_for_each_entry(rela, &relasec->relas, list) {
        struct symbol *patched_sym = rela->sym;

        if (patched_sym->twin) {
            continue;
        }
        if (sym->type != patched_sym->type ||
            (
                sym->type == STT_OBJECT &&
                sym->sym.st_size != patched_sym->sym.st_size
            )) {
            continue;
        }
        if (mangled_strcmp(patched_sym->name, sym->name)) {
            continue;
        }

        log_debug("find uncorrelated rela symbol successful %s [%s]\n",
            patched_sym->name, section_function_name(relasec));
        return patched_sym;
    }

    return NULL;
}

/*
 * Given a static local variable symbol and a rela section which references it
 * in the base object, find a corresponding usage of a similarly named symbol
 * in the patched object.
 */
static struct symbol *find_static_twin(struct section *relasec,
    struct symbol *sym)
{
    /* TODO: handle .part symbol is neccessry */
    if (!relasec->twin) {
        return NULL;
    }

    return find_uncorrelated_rela(relasec->twin, sym);
}

static struct rela *find_static_twin_ref(struct section *relasec,
    struct symbol *sym)
{
    struct rela *rela;

    list_for_each_entry(rela, &relasec->relas, list) {
        if (rela->sym == sym->twin) {
            return rela;
        }
    }

    /* TODO: handle child func here */
    return NULL;
}

/* Check two things:
 * 1. all the orig object's refercence static locals have been correlated.
 * 2. each reference to a static local in the orig object has
 *  a corresponding reference in the patched object
 *  (because a staticlocal can be referenced by more than one section)
 */
static void check_static_variable_correlate(struct upatch_elf *uelf_source,
    struct upatch_elf *uelf_patched)
{
    struct section *relasec;
    struct rela *rela;
    struct symbol *sym;

    list_for_each_entry(relasec, &uelf_source->sections, list) {
        if (!is_rela_section(relasec) ||
            is_debug_section(relasec) ||
            is_note_section(relasec)) {
            continue;
        }

        list_for_each_entry(rela, &relasec->relas, list) {
            sym = rela->sym;
            if (!is_normal_static_local(sym)) {
                continue;
            }
            if (!sym->twin || !relasec->twin) {
                log_warn("reference to static local variable %s in %s was removed",
                    sym->name, section_function_name(relasec));
                continue;
            }
            if (!find_static_twin_ref(relasec->twin, sym)) {
                ERROR("static local %s has been correlated with %s, "
                    "but patched %s is missing a reference to it",
                    sym->name, sym->twin->name,
                    section_function_name(relasec->twin));
            }
        }
    }

    /*
     * Now go through the patched object and look for any uncorrelated
     * static locals to see if we need to print any warnings about new
     * variables.
     */

    list_for_each_entry(relasec, &uelf_patched->sections, list) {
        if (!is_rela_section(relasec) ||
            is_debug_section(relasec) ||
            is_note_section(relasec)) {
            continue;
        }

        list_for_each_entry(rela, &relasec->relas, list) {
            sym = rela->sym;

            if (!is_normal_static_local(sym)) {
                continue;
            }
            if (sym->twin) {
                continue;
            }
            log_normal("unable to correlate static local variable %s used by %s"
                ", assuming variable is new\n",
                sym->name, section_function_name(relasec));
        }
    }
}

static void uncorrelate_symbol(struct symbol *sym)
{
    sym->twin->twin = NULL;
    sym->twin = NULL;
}

static void uncorrelate_section(struct section *sec)
{
    sec->twin->twin = NULL;
    sec->twin = NULL;
}

/*
 * gcc renames static local variables by appending a period and a number.  For
 * example, __foo could be renamed to __foo.31452.  Unfortunately this number
 * can arbitrarily change.  Correlate them by comparing which functions
 * reference them, and rename the patched symbols to match the base symbol
 * names.
 *
 * Some surprising facts about static local variable symbols:
 *
 * - It's possible for multiple functions to use the same
 *   static local variable if the variable is defined in an
 *   inlined function.
 *
 * - It's also possible for multiple static local variables
 *   with the same name to be used in the same function if they
 *   have different scopes.  (We have to assume that in such
 *   cases, the order in which they're referenced remains the
 *   same between the orig and patched objects, as there's no
 *   other way to distinguish them.)
 *
 * - Static locals are usually referenced by functions, but
 *   they can occasionally be referenced by data sections as
 *   well.
 */
void upatch_correlate_static_local_variables(struct upatch_elf *uelf_source,
    struct upatch_elf *uelf_patched)
{
    struct symbol *sym;
    struct symbol *patched_sym;

    struct section *relasec;
    struct rela *rela;

    int bundled;
    int patched_bundled;

    /*
     * undo the correlations for all static locals.
     * Two static locals can have the same numbered suffix in the orig
     * and patchedobjects by coincidence.
     */
    list_for_each_entry(sym, &uelf_source->symbols, list) {
        if (!is_normal_static_local(sym)) {
            continue;
        }

        if (sym->twin) {
            uncorrelate_symbol(sym);
        }

        bundled = (sym == sym->sec->bundle_sym) ? 1 : 0;
        if (bundled && sym->sec->twin) {
            uncorrelate_section(sym->sec);
            if (sym->sec->sym) {
                uncorrelate_symbol(sym->sec->sym);
            }
            if (sym->sec->rela) {
                // uncorrelate relocation section which not equals to reference
                uncorrelate_section(sym->sec->rela);
            }
        }
    }

    /*
     * Do the correlations: for each section reference to a static local,
     * look for a corresponding reference in the section's twin.
     */
    list_for_each_entry(relasec, &uelf_source->sections, list) {
        /* handle .rela.toc sectoins */
        if (!is_rela_section(relasec) ||
            is_debug_section(relasec) ||
            is_note_section(relasec)) {
            continue;
        }

        /* check all relocation symbols */
        list_for_each_entry(rela, &relasec->relas, list) {
            sym = rela->sym;

            if (!is_normal_static_local(sym)) {
                continue;
            }
            if (sym->twin) {
                continue;
            }

            bundled = (sym == sym->sec->bundle_sym) ? 1 : 0;
            if (bundled && sym->sec == relasec->base) {
                /*
                 * TODO:
                 * A rare case where a static local data structure references
                 * itself.
                 * There's no reliable way to correlate this.
                 * Hopefully to the symbol somewhere that can be used.
                 */
                log_debug("can't correlate static local %s's ref to itself\n",
                    sym->name);
                continue;
            }

            patched_sym = find_static_twin(relasec, sym);
            if (!patched_sym) {
                log_warn("reference to static local variable %s in %s was removed",
                    sym->name, section_function_name(relasec));
                continue;
            }

            patched_bundled = (patched_sym == patched_sym->sec->bundle_sym) ? 1 : 0;
            if (bundled != patched_bundled) {
                ERROR("bundle mismatch for symbol %s", sym->name);
            }
            if (!bundled && sym->sec->twin && sym->sec->twin != patched_sym->sec) {
                ERROR("sections %s and %s aren't correlated for symbol %s",
                    sym->sec->name, patched_sym->sec->name, sym->name);
            }

            correlate_symbol(sym, patched_sym);
            if (bundled) {
                correlate_section(sym->sec, patched_sym->sec);
            }
        }
    }

    return check_static_variable_correlate(uelf_source, uelf_patched);
}
