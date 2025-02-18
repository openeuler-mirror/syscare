// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
 * Copyright (C) 2024 Huawei Technologies Co., Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#include "upatch-relocation.h"
#include <errno.h>

#include "log.h"

int apply_relocations(struct upatch_elf *uelf)
{
    unsigned int i;
    int err = 0;

    /* Now do relocations. */
    for (i = 1; i < uelf->info.hdr->e_shnum; i++) {
        unsigned int infosec = uelf->info.shdrs[i].sh_info;
        const char *name = uelf->info.shstrtab + uelf->info.shdrs[i].sh_name;

        /* Not a valid relocation section? */
        if (infosec >= uelf->info.hdr->e_shnum) {
            continue;
        }

        /* Don't bother with non-allocated sections */
        if (!(uelf->info.shdrs[infosec].sh_flags & SHF_ALLOC)) {
            continue;
        }

        log_debug("Relocate section '%s':\n", name);
        if (uelf->info.shdrs[i].sh_type == SHT_REL) {
            return -EPERM;
        } else if (uelf->info.shdrs[i].sh_type == SHT_RELA) {
            err = apply_relocate_add(uelf, uelf->index.sym, i);
        }
        log_debug("\n");

        if (err < 0) {
            break;
        }
    }

    return err;
}
