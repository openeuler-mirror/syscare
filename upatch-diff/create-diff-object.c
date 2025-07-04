// SPDX-License-Identifier: GPL-2.0
/*
 * create-diff-object.c
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

/*
 * This file contains the heart of the ELF object differencing engine.
 *
 * The tool takes two ELF objects from two versions of the same source
 * file; a "orig" object and a "patched" object.  These object need to have
 * been compiled with the -ffunction-sections and -fdata-sections GCC options.
 *
 * The tool compares the objects at a section level to determine what
 * sections have changed.  Once a list of changed sections has been generated,
 * various rules are applied to determine any object local sections that
 * are dependencies of the changed section and also need to be included in
 * the output object.
 */

#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <errno.h>
#include <libgen.h>
#include <argp.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/stat.h>

#include "log.h"
#include "elf-debug.h"
#include "elf-common.h"
#include "elf-insn.h"
#include "elf-compare.h"
#include "elf-correlate.h"
#include "elf-resolve.h"
#include "elf-create.h"
#include "running-elf.h"
#include "upatch-patch.h"

#define PROG_VERSION "upatch-diff "BUILD_VERSION

enum log_level g_loglevel = NORMAL;
char *g_logprefix;
char *g_uelf_name;
char *g_relf_name;

struct arguments {
    char *source_obj;
    char *patched_obj;
    char *running_elf;
    char *output_obj;
    unsigned long text_offset;
    bool debug;
};

static const struct argp_option ARGP_OPTION[] = {
    {"source",      's', "<file>",   0, "Source object",       0},
    {"patched",     'p', "<file>",   0, "Patched object",      1},
    {"running",     'r', "<file>",   0, "Running binary file", 2},
    {"output",      'o', "<file>",   0, "Output object",       3},
    {"text-offset", 't', "<offset>", 0, "Text section offset", 4},
    {"debug",       'd', NULL,       0, "Show debug output",   5},
    {NULL}
};
static const char ARGP_DOC[] = "Generate a patch object based on source object";
const char *argp_program_version = PROG_VERSION;

static void parse_text_offset(struct argp_state *state, const char *arg)
{
    errno = 0;
    char *endptr = NULL;

    unsigned long offset = strtoul(arg, &endptr, 0);
    if ((errno != 0) || (*endptr != '\0') ||
        ((errno == ERANGE) && (offset == ULONG_MAX))) {
        argp_error(state, "ERROR: Invalid text section offset '%s'", arg);
    }

    struct arguments *arguments = state->input;
    arguments->text_offset = offset;
}

static error_t parse_opt(int key, char *arg, struct argp_state *state)
{
    struct arguments *arguments = state->input;

    switch (key) {
        case 's':
            arguments->source_obj = arg;
            break;
        case 'p':
            arguments->patched_obj = arg;
            break;
        case 'r':
            arguments->running_elf = arg;
            break;
        case 'o':
            arguments->output_obj = arg;
            break;
        case 't':
            parse_text_offset(state, arg);
            break;
        case 'd':
            arguments->debug = true;
            break;
        default:
            return ARGP_ERR_UNKNOWN;
    }
    return 0;
}

static bool check_args(struct arguments *arguments)
{
    if (arguments->source_obj == NULL) {
        log_error("The argument '--source <file>' requires a value\n");
        return false;
    }
    if (arguments->patched_obj == NULL) {
        log_error("The argument '--patched <file>' requires a value\n");
        return false;
    }
    if (arguments->running_elf == NULL) {
        log_error("The argument '--running <file>' requires a value\n");
        return false;
    }
    if (arguments->output_obj == NULL) {
        log_error("The argument '--output <file>' requires a value\n");
        return false;
    }
    if (arguments->text_offset > UINT32_MAX) {
        ERROR("Text section offset 0x%lx overflow", arguments->text_offset);
    }
    if ((arguments->text_offset & 0xFFF) != 0) {
        ERROR("Text section offset 0x%lx is not 4K-aligned",
            arguments->text_offset);
    }
    return true;
}

static void show_program_info(struct arguments *arguments)
{
    log_debug("==============================\n");
    log_debug("%s\n", PROG_VERSION);
    log_debug("==============================\n");
    log_debug("source object:  %s\n", arguments->source_obj);
    log_debug("patched object: %s\n", arguments->patched_obj);
    log_debug("running binary: %s\n", arguments->running_elf);
    log_debug("output object:  %s\n", arguments->output_obj);
    log_debug("text offset:    0x%lx\n", arguments->text_offset);
    log_debug("------------------------------\n\n");
}

static void compare_elf_headers(struct upatch_elf *uelf_source,
    struct upatch_elf *uelf_patched)
{
    GElf_Ehdr ehdr_source, ehdr_patched;

    if (!gelf_getehdr(uelf_source->elf, &ehdr_source)) {
        ERROR("gelf_getehdr source failed for %s.", elf_errmsg(0));
    }
    if (!gelf_getehdr(uelf_patched->elf, &ehdr_patched)) {
        ERROR("gelf_getehdr patched failed for %s.", elf_errmsg(0));
    }
    if (memcmp(ehdr_source.e_ident, ehdr_patched.e_ident, EI_NIDENT) ||
        ehdr_source.e_type != ehdr_patched.e_type ||
        ehdr_source.e_machine != ehdr_patched.e_machine ||
        ehdr_source.e_version != ehdr_patched.e_version ||
        ehdr_source.e_entry != ehdr_patched.e_entry ||
        ehdr_source.e_phoff != ehdr_patched.e_phoff ||
        ehdr_source.e_flags != ehdr_patched.e_flags ||
        ehdr_source.e_ehsize != ehdr_patched.e_ehsize ||
        ehdr_source.e_phentsize != ehdr_patched.e_phentsize ||
        ehdr_source.e_shentsize != ehdr_patched.e_shentsize) {
        ERROR("compare_elf_headers failed.");
    }
}

static char *strarrcmp(char *name, char **prefix)
{
    size_t len;

    if (name == NULL) {
        return NULL;
    }
    while (*prefix != NULL) {
        len = strlen(*prefix);
        if (!strncmp(name, *prefix, len)) {
            return name + len;
        }
        prefix++;
    }

    return NULL;
}

static bool is_bundleable(struct symbol *sym)
{
    char *name = NULL;
    size_t text_name_len = 0;
    /* handle .text.unlikely. and then .text. */
    char *func_prefix[] = {
        ".text.unlikely.",
        ".text.startup.", /* used for cold startup main function */
        ".text.hot.",
        ".text.",
        NULL,
    };

    char *obj_prefix[] = {
        ".data.rel.ro.",
        ".data.rel.",
        ".data.",
        ".rodata.",
        ".bss.",
        NULL,
    };

    if (sym == NULL || sym->sec == NULL) {
        return false;
    }

    if (sym->type == STT_FUNC) {
        name = strarrcmp(sym->sec->name, func_prefix);
    } else if (sym->type == STT_OBJECT) {
        name = strarrcmp(sym->sec->name, obj_prefix);
    }

    /* no prefix found or invalid type */
    if (name == NULL) {
        return false;
    }
    if (!strcmp(name, sym->name)) {
        return true;
    }

    /* special case for cold func */
    text_name_len = strlen(".text.unlikely.");
    if (sym->type == STT_FUNC &&
        !strncmp(sym->sec->name, ".text.unlikely.", text_name_len) &&
        strstr(sym->name, ".cold") &&
        !strncmp(sym->sec->name + text_name_len, sym->name,
            strlen(sym->sec->name) - text_name_len)) {
        return true;
    }

    return false;
}

/*
 * When compiled with -ffunction-sections and -fdata-sections, almost each
 * symbol gets its own dedicated section. We call such symbols "bundled"
 * symbols. It can be checked by "sym->sec->sym == sym"
 */
static void bundle_symbols(struct upatch_elf *uelf)
{
    struct symbol *sym;

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (is_bundleable(sym)) {
            if (sym->sym.st_value != 0 &&
                is_gcc6_localentry_bundled_sym(uelf)) {
                ERROR("Symbol '%s' at offset %lu of section '%s', expected 0.",
                    sym->name, sym->sym.st_value, sym->sec->name);
            }
            sym->sec->bundle_sym = sym;
        /* except handler is also a kind of bundle symbol */
        } else if (sym->type == STT_SECTION && is_except_section(sym->sec)) {
            sym->sec->bundle_sym = sym;
        }
    }
}

/*
 * During optimization, gcc may move unlikely execution branches into *.cold
 * subfunctions. Some functions can also be split into mutiple *.part funtions.
 * detect_child_functions detects such subfunctions and crossreferences
 * them with their parent functions through parent/child pointers.
 */
static void detect_child_functions(struct upatch_elf *uelf)
{
    struct symbol *sym;
    char *childstr;
    char *pname;

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->type != STT_FUNC) {
            continue;
        }
        childstr = strstr(sym->name, ".cold");
        if (!childstr) {
            childstr = strstr(sym->name, ".part");
        }
        if (!childstr) {
            continue;
        }

        pname = strndup(sym->name, (size_t)(childstr - sym->name));
        log_debug("symbol '%s', pname: '%s'\n", sym->name, pname);
        if (!pname) {
            ERROR("detect_child_functions strndup failed.");
        }

        sym->parent = find_symbol_by_name(&uelf->symbols, pname);
        if (sym->parent) {
            list_add_tail(&sym->subfunction_node, &sym->parent->children);
        }

        free(pname);
    }
}

static void mark_file_symbols(struct upatch_elf *uelf)
{
    struct symbol *curr_sym = NULL;
    struct symbol *file_sym = NULL;

    list_for_each_entry(curr_sym, &uelf->symbols, list) {
        if (curr_sym->type == STT_FILE) {
            file_sym = curr_sym;
            continue;
        }
        if ((file_sym == NULL) || (file_sym->status == CHANGED)) {
            continue;
        }
        if (curr_sym->status == CHANGED) {
            file_sym->status = CHANGED;
        }
    }
}

static void mark_grouped_sections(struct upatch_elf *uelf)
{
    struct section *groupsec;
    list_for_each_entry(groupsec, &uelf->sections, list) {
        if (groupsec->sh.sh_type != SHT_GROUP) {
            continue;
        }

        GElf_Word *data = groupsec->data->d_buf;
        GElf_Word *end = groupsec->data->d_buf + groupsec->data->d_size;
        data++; /* skip first flag word (e.g. GRP_COMDAT) */

        while (data < end) {
            struct section *sec = find_section_by_index(&uelf->sections, (GElf_Section)*data);
            if (sec == NULL) {
                ERROR("Cannot find group section, index=%d", *data);
            }
            sec->grouped = true;
            log_debug("Marking grouped section, index: %d, name: '%s'\n",
                sec->index, sec->name);
            data++;
        }
    }
}

/*
 * There are two kinds of relocation. One is based on the variable symbol.
 * And the other one is based on the section symbol. The second type is often
 * used for static objects. Here, we replace the second type with the first ons.
 * So we can compare them with each other directly.
 */
static void replace_section_syms(struct upatch_elf *uelf)
{
    struct section *relasec;
    struct rela *rela;
    struct symbol *sym;
    long target_off;
    bool found = false;

    list_for_each_entry(relasec, &uelf->sections, list) {
        if (!is_rela_section(relasec) ||
            is_debug_section(relasec) ||
            is_note_section(relasec)) {
            continue;
        }

        list_for_each_entry(rela, &relasec->relas, list) {
            if (!rela->sym || !rela->sym->sec ||
                rela->sym->type != STT_SECTION) {
                continue;
            }
            /*
             * for section symbol, rela->sym->sec is the section itself.
             * rela->sym->sec->sym is the bundleable symbol which is
             * a function or object.
             */
            if (rela->sym->sec->bundle_sym) {
                rela->sym = rela->sym->sec->bundle_sym;
                if (rela->sym->sym.st_value != 0) {
                    ERROR("Symbol offset is not zero.");
                }
                continue;
            }

            target_off = rela_target_offset(uelf, relasec, rela);
            list_for_each_entry(sym, &uelf->symbols, list) {
                long start, end;

                /*
                 * find object which belongs to this section,
                 * it could be .data .rodata etc.
                 */
                if (sym->type == STT_SECTION || sym->sec != rela->sym->sec) {
                    continue;
                }

                start = (long)sym->sym.st_value;
                end = (long)(sym->sym.st_value + sym->sym.st_size);

                /* text section refer other sections */
                if (is_text_section(relasec->base) &&
                    !is_text_section(sym->sec) &&
                    (rela->type == R_X86_64_32S ||
                        rela->type == R_X86_64_32 ||
                        rela->type == R_AARCH64_ABS64 ||
                        rela->type == R_RISCV_64) &&
                    rela->addend == (long)sym->sec->sh.sh_size &&
                    end == (long)sym->sec->sh.sh_size) {
                    ERROR("Relocation refer end of data sections.");
                } else if (target_off == start && target_off == end) {
                    if (is_mapping_symbol(uelf, sym)) {
                        continue;
                    }
                } else if (target_off < start || target_off >= end) {
                    continue;
                }

                found = true;
                rela->sym = sym;
                rela->addend -= start;
                break;
            }

            /* only rodata and data based is allowed
             * if we compile with fPIC and the function's local char* array is too large,
             * (we test the array's size > 32),
             * gcc will generate the relocation rodata.str1.1 about the array in .data section.
             * this .data symbol's type is STT_SECTION. and this function has the .data
             * symbol's relocation. just like:
             *
             * code:
             * int glo_func(void)
             * {
             * char *help[]={"test1", "test2",.....,"test33"};
             * return 0;
             * }
             *
             * elf:
             * Relocation section '.rela.data' at offset 0xc30 contains 33 entries:
             * Offset          Info           Type           Sym. Value    Sym. Name + Addend
             * 000000000000  000300000001 R_X86_64_64       0000000000000000 .rodata.str1.1 + 0
             * 000000000008  000300000001 R_X86_64_64       0000000000000000 .rodata.str1.1 + 6
             * ....
             *
             * Relocation section '.rela.text.glo_func' at offset 0x738 contains 3 entries:
             * Offset          Info           Type           Sym. Value    Sym. Name + Addend
             * 000000000015  000200000002 R_X86_64_PC32     0000000000000000 .data - 4
             *
             * but if we change the other function which has nothing to do with this .data
             * section and the glo_function. the glo_function will still error because of
             * the glo_function's .data relocation.
             *
             * we do not allow .data section is "include" in verify_patchability. so we
             * don't worry about the .data section will produce unexpected behavior later on.
             */
            if (!found && !is_string_literal_section(rela->sym->sec) &&
                strncmp(rela->sym->name, ".rodata", strlen(".rodata")) &&
                strncmp(rela->sym->name, ".data", strlen(".data"))) {
                ERROR("%s+0x%lx: Cannot find replacement symbol for '%s+%ld' reference.",
                relasec->base->name, rela->offset, rela->sym->name, rela->addend);
            }
        }
    }
}

static void mark_ignored_sections(struct upatch_elf *uelf)
{
    static const char *const IGNORED_SECTIONS[] = {
        ".eh_frame",
        ".note",
        ".debug_",
        ".comment",
        ".discard",
        ".rela.discard",
        ".GCC.command.line",
    };
    static const size_t IGNORED_SECTION_NUM =
        sizeof(IGNORED_SECTIONS) / sizeof(IGNORED_SECTIONS[0]);

    struct section *sec = NULL;
    list_for_each_entry(sec, &uelf->sections, list) {
        for (size_t i = 0; i < IGNORED_SECTION_NUM; i++) {
            const char *const ignored_name = IGNORED_SECTIONS[i];
            const size_t name_len = strlen(ignored_name);
            const char *sec_name = is_rela_section(sec) ?
                sec->base->name : sec->name;
            if (strncmp(sec_name, ignored_name, name_len) == 0) {
                sec->ignored = true;
                log_debug("Marking ignored section, index: %d, name: '%s'\n",
                    sec->index, sec->name);
                break;
            }
        }
    }
}
/*
* For a local symbol referenced in the rela list of a changing function,
* if it has no section, it will link error in arm.
* So we create a empty section for link purpose.
* We use st_other to mark these symbols.
*/
static void include_special_local_section(struct upatch_elf *uelf) {
    struct symbol *sym;
    struct symbol *sym_changed;
    struct rela *rela;

    list_for_each_entry(sym_changed, &uelf->symbols, list) {
        if (!(sym_changed->status == CHANGED && sym_changed->type == STT_FUNC)) {
            continue;
        }
        if (!sym_changed->sec || !sym_changed->sec->rela) {
            continue;
        }

        list_for_each_entry(rela, &sym_changed->sec->rela->relas, list) {
            sym = rela->sym;
            if (sym->sec && sym->bind == STB_LOCAL &&
                sym->status == SAME && !sym->sec->include) {
                sym->sym.st_other |= SYM_OTHER;
                sym->sec->include = true;
                sym->sec->data->d_buf = NULL;
                sym->sec->data->d_size = 0;
                // arm error: (.debug_info+0x...) undefined reference to `no symbol'
                if (sym->sec->sym) {
                    sym->sec->sym->include = true;
                }
            }
        }
    }
}

static void include_section(struct section *sec);
static void include_symbol(struct symbol *sym)
{
    if ((sym == NULL) || sym->include) {
        return;
    }
    /*
     * The symbol gets included even if its section isn't needed, as it
     * might be needed: either permanently for a rela, or temporarily for
     * the later creation of a dynrela.
     */
    sym->include = true;
    /*
     * For special static symbols, we need include it's section
     * to ensure we don't get link error.
     */
    if (is_special_static_symbol(sym)) {
        sym->sec->include = true;
    }
    /*
     * For a function/object symbol, if it has a section, we only need to
     * include the section if it has changed. Otherwise the symbol will be
     * used by relas/dynrelas to link to the real symbol externally.
     *
     * For section symbols, we always include the section because
     * references to them can't otherwise be resolved externally.
     */
    if ((sym->status != SAME) || (sym->type == STT_SECTION)) {
        include_section(sym->sec);
    }
#ifdef __riscv
    /* .L symbols not exist in EXE. If they are included, so are their sections. */
    else if (sym->sec && !sym->sec->include && !strncmp(sym->name, ".L", 2)) {
        include_section(sym->sec);
    }
#endif
}

static void include_section(struct section *sec)
{
    if ((sec == NULL) || sec->include) {
        return;
    }

    sec->include = true;

    if (is_rela_section(sec)) {
        struct rela *rela = NULL;
        list_for_each_entry(rela, &sec->relas, list) {
            include_symbol(rela->sym);
        }
        return;
    } else {
        include_symbol(sec->sym);
        include_section(sec->rela);
    }
}

static void include_standard_elements(struct upatch_elf *uelf)
{
    struct section *sec = NULL;

    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->ignored) {
            continue;
        }
        if (is_symtab_section(sec) || is_strtab_section(sec)) {
            include_section(sec);
        }
    }

    /* include the NULL symbol */
    struct symbol *sym = find_symbol_by_index(&uelf->symbols, 0);
    if (sym == NULL) {
        ERROR("Cannot find null symbol");
    }
    include_symbol(sym);
}

static int include_changes(struct upatch_elf *uelf)
{
    int count = 0;

    struct symbol *sym = NULL;
    list_for_each_entry(sym, &uelf->symbols, list) {
        if ((sym->status == SAME) || is_symbol_ignored(sym)) {
            continue;
        }

        if ((sym->type == STT_OBJECT) ||
            (sym->type == STT_FUNC) ||
            (sym->type == STT_COMMON) ||
            (sym->type == STT_TLS) ||
            (sym->type == STT_GNU_IFUNC)) {
            include_symbol(sym);
            count++;
        } else if (sym->type == STT_SECTION) {
            if ((sym->sec != NULL) && is_rela_section(sym->sec)) {
                continue;
            }
            include_symbol(sym);
            count++;
        }
    }

    return count;
}

static int verify_symbol_patchability(struct upatch_elf *uelf)
{
    int err_count = 0;

    struct symbol *sym = NULL;
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (!sym->include) {
            continue;
        }
        if ((sym->bind == STB_LOCAL) && (sym->sym.st_shndx == SHN_UNDEF) &&
            (sym->index != 0)) {
            log_warn("Symbol '%s' is local, but sh_shndx is SHN_UNDEF\n",
                sym->name);
            err_count++;
        }
        if (sym->type == STT_GNU_IFUNC) {
            log_warn("Symbol '%s' is included, but IFUNC is not supported\n",
                sym->name);
            err_count++;
        }
    }

    return err_count;
}

static int verify_section_patchability(struct upatch_elf *uelf)
{
    int err_count = 0;

    struct section *sec = NULL;
    list_for_each_entry(sec, &uelf->sections, list) {
        if (sec->ignored) {
            continue;
        }
        if ((sec->status == NEW) && !sec->include) {
            // new sections should be included
            log_warn("Section '%s' is %s, but it is not included\n",
                sec->name, status_str(sec->status));
            err_count++;
        } else if ((sec->status == CHANGED) && !sec->include) {
            // changed sections should be included
            if (is_rela_section(sec)) {
                continue;
            }
            log_warn("Section '%s' is %s, but it is not included\n",
                sec->name, status_str(sec->status));
            err_count++;
        } else if ((sec->status == CHANGED) && sec->include) {
            // changed group section cannot be included
            if (is_group_section(sec) || sec->grouped) {
                log_warn("Section '%s' is %s, but it is not supported\n",
                    sec->name, status_str(sec->status));
                err_count++;
            }
            // changed .data & .bss section cannot be included
            if (is_data_section(sec) || is_bss_section(sec)) {
                struct rela *rela = NULL;
                list_for_each_entry(rela, &sec->rela->relas, list) {
                    if ((rela->sym == NULL) || (rela->sym->status != CHANGED)) {
                        continue;
                    }
                    if (is_read_only_section(rela->sym->sec) ||
                        is_string_literal_section(rela->sym->sec)) {
                        continue;
                    }
                    log_warn("Section '%s' is %s, but it is not supported\n",
                        sec->name, status_str(sec->status));
                    err_count++;
                }
            }
        }
    }

    return err_count;
}

static void verify_patchability(struct upatch_elf *uelf)
{
    int err_count = 0;

    err_count += verify_symbol_patchability(uelf);
    err_count += verify_section_patchability(uelf);
    if (err_count != 0) {
        ERROR("Found %d unexpected changes", err_count);
    }
}

/*
 * These types are for linker optimization and memory layout.
 * They have no associated symbols and their names are empty
 * string which would mismatch running-elf symbols in later
 * lookup_relf(). Drop these useless items now.
 */
static void rv_drop_useless_rela(struct section *relasec)
{
    struct rela *rela, *saferela;
    list_for_each_entry_safe(rela, saferela, &relasec->relas, list)
        if (rela->type == R_RISCV_RELAX || rela->type == R_RISCV_ALIGN) {
            list_del(&rela->list);
            memset(rela, 0, sizeof(*rela));
            free(rela);
        }
}

static void migrate_included_elements(struct upatch_elf *uelf_patched,
    struct upatch_elf *uelf_out)
{
    struct section *sec;
    struct section *safesec;
    struct symbol *sym;
    struct symbol *safesym;

    uelf_out->arch = uelf_patched->arch;

    INIT_LIST_HEAD(&uelf_out->sections);
    INIT_LIST_HEAD(&uelf_out->symbols);
    INIT_LIST_HEAD(&uelf_out->strings);

    /* migrate included sections from uelf_patched to uelf_out */
    list_for_each_entry_safe(sec, safesec, &uelf_patched->sections, list) {
        if (!sec->include) {
            continue;
        }
        list_del(&sec->list);
        list_add_tail(&sec->list, &uelf_out->sections);
        sec->index = 0;

        if (!is_rela_section(sec)) {
            if (sec->sym && !sec->sym->include) {
                sec->sym = NULL; // break link to non-included section symbol
            }
        } else if (uelf_patched->arch == RISCV64) {
            rv_drop_useless_rela(sec);
        }
    }

    /* migrate included symbols from kelf to out */
    list_for_each_entry_safe(sym, safesym, &uelf_patched->symbols, list) {
        if (!sym->include) {
            continue;
        }

        list_del(&sym->list);
        list_add_tail(&sym->list, &uelf_out->symbols);
        sym->index = 0;
        sym->strip = false;

        if (sym->sec && !sym->sec->include) {
            sym->sec = NULL; // break link to non-included section
        }
    }
}

/*
 * Key point for upatch-diff:
 * 1. find changed func/data for each object
 * 2. link all these objects into a relocatable file
 * 3. add sections for management (hash/init/patch info etc.)
 * 4. locate old symbols for the relocatable file
 */
int main(int argc, char **argv)
{
    static const struct argp ARGP = {
        ARGP_OPTION, parse_opt, NULL, ARGP_DOC, NULL, NULL, NULL
    };
    struct arguments args = { 0 };
    struct upatch_elf uelf_source = { 0 };
    struct upatch_elf uelf_patched = { 0 };
    struct upatch_elf uelf_out = { 0 };
    struct running_elf relf = { 0 };

    if (argp_parse(&ARGP, argc, argv, 0, NULL, &args) != 0) {
        return EXIT_FAILURE;
    }
    if (!check_args(&args)) {
        return EXIT_FAILURE;
    }
    if (args.debug) {
        g_loglevel = DEBUG;
    }
    show_program_info(&args);

    if (elf_version(EV_CURRENT) == EV_NONE) {
        log_error("Failed to initialize elf library\n");
        return EXIT_FAILURE;
    }

    uelf_open(&uelf_source, args.source_obj);
    uelf_open(&uelf_patched, args.patched_obj);
    relf_open(&relf, args.running_elf);

    g_logprefix = basename(args.source_obj);
    g_uelf_name = args.source_obj;
    g_relf_name = args.running_elf;
    compare_elf_headers(&uelf_source, &uelf_patched);

    bundle_symbols(&uelf_source);
    bundle_symbols(&uelf_patched);

    detect_child_functions(&uelf_source);
    detect_child_functions(&uelf_patched);

    mark_ignored_sections(&uelf_source);
    mark_ignored_sections(&uelf_patched);
    mark_grouped_sections(&uelf_patched);

    replace_section_syms(&uelf_source);
    replace_section_syms(&uelf_patched);

    upatch_correlate_elf(&uelf_source, &uelf_patched);
    upatch_correlate_static_local_variables(&uelf_source, &uelf_patched);
    upatch_print_correlation(&uelf_patched);

    upatch_compare_correlated_elements(&uelf_patched);
    mark_file_symbols(&uelf_source);

    include_standard_elements(&uelf_patched);
    int change_count = include_changes(&uelf_patched);
    if (change_count == 0) {
        log_normal("No functional changes\n");
        uelf_close(&uelf_source);
        uelf_close(&uelf_patched);
        relf_close(&relf);
        return 0;
    }
    upatch_print_changes(&uelf_patched);

    verify_patchability(&uelf_patched);

    include_special_local_section(&uelf_patched);

    migrate_included_elements(&uelf_patched, &uelf_out);

    upatch_create_strings_elements(&uelf_out);

    upatch_create_patches_sections(&uelf_out, &relf, args.text_offset);

    create_kpatch_arch_section();

    upatch_build_strings_section_data(&uelf_out);

    /*
     * At this point, the set of output sections and symbols is finalized.
     * Reorder eth symbols into link-compliant order and index all the symbols
     * and sections. After the indexes have beed established, update index data
     * throughout the structure.
     */
    upatch_reorder_symbols(&uelf_out);

    upatch_strip_unneeded_syms(&uelf_out);

    upatch_reindex_elements(&uelf_out);

    upatch_rebuild_relocations(&uelf_out);

    upatch_create_shstrtab(&uelf_out);

    upatch_create_strtab(&uelf_out);

    upatch_partly_resolve(&uelf_out, &relf);

    upatch_create_symtab(&uelf_out);

    upatch_write_output_elf(&uelf_out, uelf_patched.elf, args.output_obj, 0664);
    log_normal("Done\n");

    uelf_close(&uelf_out);
    uelf_close(&uelf_patched);
    uelf_close(&uelf_source);
    relf_close(&relf);

    fflush(stdout);
    fflush(stderr);
    return EXIT_SUCCESS;
}
