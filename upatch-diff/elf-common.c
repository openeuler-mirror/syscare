// SPDX-License-Identifier: GPL-2.0
/*
 * elf-common.c
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

#ifdef __riscv
/*
 * .L local symbols are named as ".L" + "class prefix" + "number".
 * The numbers are volatile due to code change.
 * Compare class prefix(composed of letters) only.
 */
static int mangled_strcmp_dot_L(char *str1, char *str2)
{
    if (!*str2 || strncmp(str2, ".L", 2)) {
        return 1;
    }

    /* RISCV_FAKE_LABEL_NAME matched exactly */
    if (!strcmp(str1, ".L0 ") || !strcmp(str2, ".L0 ")) {
        return strcmp(str1, str2);
    }

    char *p = str1 + 2;
    char *q = str2 + 2;
    while (*p < '0' || *p > '9') p++;
    while (*q < '0' || *q > '9') q++;
    if ((p - str1 != q - str2) || strncmp(str1, str2, (size_t)(p - str1))) {
        return 1;
    }

    return 0;
}
#endif

static bool is_dynamic_debug_symbol(struct symbol *sym)
{
    static const char *SEC_NAMES[] = {
        "__verbose",
        "__dyndbg",
        NULL,
    };

    if ((sym->type == STT_OBJECT) || (sym->type == STT_SECTION)) {
        const char **sec_name;
        for (sec_name = SEC_NAMES; *sec_name; sec_name++) {
            if (strcmp(sym->sec->name, *sec_name) == 0) {
                return true;
            }
        }
    }

    return false;
}

bool is_special_static_symbol(struct symbol *sym)
{
    static const char *SYM_NAMES[] = {
        ".__key",
        ".__warned",
        ".__already_done.",
        ".__func__",
        ".__FUNCTION__",
        ".__PRETTY_FUNCTION__",
        "._rs",
        ".CSWTCH",
        "._entry",
        ".C",
        NULL,
    };

    if (sym == NULL) {
        return false;
    }

    /* pr_debug() uses static local variables in __verbose or __dyndbg section */
    if (is_dynamic_debug_symbol(sym)) {
        return true;
    }

    if (sym->type == STT_SECTION) {
        /* make sure section is bundled */
        if (is_rela_section(sym->sec) || (sym->sec->sym == NULL)) {
            return false;
        }
        /* use bundled object object/function symbol for matching */
        sym = sym->sec->sym;
    }

    if ((sym->type != STT_OBJECT) || (sym->bind != STB_LOCAL)) {
        return false;
    }
    if (!strcmp(sym->sec->name, ".data.once")) {
        return true;
    }

    const char **sym_name;
    for (sym_name = SYM_NAMES; *sym_name; sym_name++) {
        /* Check gcc-style statics: '<sym_name>.' */
        if (strcmp(sym->name, (*sym_name + 1)) == 0) {
            return true;
        }
        /* Check clang-style statics: '<function_name>.<sym_name>' */
        if (strstr(sym->name, *sym_name)) {
            return true;
        }
    }

    return false;
}

bool is_special_static_section(struct section *sec)
{
    struct symbol *sym = is_rela_section(sec) ?
        sec->base->secsym : sec->secsym;
    return is_special_static_symbol(sym);
}

int mangled_strcmp(char *str1, char *str2)
{
    /*
    * ELF string sections aren't mangled, though they look that way.
    */
    if (strstr(str1, ".str1.")) {
        return strcmp(str1, str2);
    }

#ifdef __riscv
    if (!strncmp(str1, ".L", 2)) {
        return mangled_strcmp_dot_L(str1, str2);
    }
#endif

    while (*str1 == *str2) {
        if (!*str2) {
            return 0;
        }
        // format like ".[0-9]"
        if (*str1 == '.' && isdigit(str1[1])) {
            if (!isdigit(str2[1])) {
                return 1;
            }
            while (isdigit(*++str1)) {
                // empty loop body
            }
            while (isdigit(*++str2)) {
                // empty loop body
            }
        } else {
            str1++;
            str2++;
        }
    }

    if ((!*str1 && has_digit_tail(str2)) ||
        (!*str2 && has_digit_tail(str1))) {
        return 0;
    }

    return 1;
}

bool is_normal_static_local(struct symbol *sym)
{
    // only handle local variable
    if (sym->type != STT_OBJECT || sym->bind != STB_LOCAL) {
        return false;
    }
    // TODO: .Local ? need a example here
    if (!strncmp(sym->name, ".L", 2)) {
        ERROR("find no-local variable\n");
        return false;
    }
    if (!strchr(sym->name, '.')) {
        return false;
    }
    if (is_special_static_symbol(sym)) {
        return false;
    }
    return true;
}

int offset_of_string(struct list_head *list, char *name)
{
    struct string *string;
    int index = 0;

    list_for_each_entry(string, list, list) {
        if (!strcmp(string->name, name)) {
            return index;
        }
        index += (int)strlen(string->name) + 1;
    }

    ALLOC_LINK(string, list);
    string->name = name;
    return index;
}

// no need for X86
bool is_gcc6_localentry_bundled_sym(struct upatch_elf *uelf)
{
    switch (uelf->arch) {
        case AARCH64:
            return false;
        case X86_64:
            return false;
        case RISCV64:
            return false;
        default:
            ERROR("unsupported arch");
    }
    return false;
}

bool is_mapping_symbol(struct upatch_elf *uelf, struct symbol *sym)
{
    if ((uelf->arch != AARCH64) && (uelf->arch != RISCV64)) {
        return false;
    }
    if (sym->name && sym->name[0] == '$' &&
        sym->type == STT_NOTYPE &&
        sym->bind == STB_LOCAL) {
        return true;
    }

    return false;
}
