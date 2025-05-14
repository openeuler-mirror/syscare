// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-resolve.h
 *
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

#ifndef __UPATCH_RESOLVE_H_
#define __UPATCH_RESOLVE_H_

struct upatch_elf;
struct running_elf;

void upatch_partly_resolve(struct upatch_elf *, struct running_elf *);

#endif /* __UPATCH_RESOLVE_H_ */
