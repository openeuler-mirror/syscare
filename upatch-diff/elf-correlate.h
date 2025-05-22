// SPDX-License-Identifier: GPL-2.0
/*
 * elf-correlate.h
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

#ifndef __UPATCH_ELF_CORRELATE_H_
#define __UPATCH_ELF_CORRELATE_H_

#include "upatch-elf.h"

void upatch_correlate_sections(struct upatch_elf *, struct upatch_elf *);

void upatch_correlate_symbols(struct upatch_elf *, struct upatch_elf *);

static inline void upatch_correlate_elf(struct upatch_elf *uelf_source, struct upatch_elf *uelf_patched)
{
    upatch_correlate_sections(uelf_source, uelf_patched);
    upatch_correlate_symbols(uelf_source, uelf_patched);
}

void upatch_correlate_static_local_variables(struct upatch_elf *, struct upatch_elf *);

#endif
