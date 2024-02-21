// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-hijacker kernel module
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

#ifndef _UPATCH_HIJACKER_KO_UPROBE_H
#define _UPATCH_HIJACKER_KO_UPROBE_H

#include <linux/types.h>

struct uprobe_consumer;
struct pt_regs;

int handle_uprobe(struct uprobe_consumer *self, struct pt_regs *regs);

#endif /* _UPATCH_HIJACKER_KO_UPROBE_H */
