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

    /*
     * TODO: Special static local variables should never be correlated and should always
     * be included if they are referenced by an included function.
     */
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
