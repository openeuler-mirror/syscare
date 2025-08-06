// SPDX-License-Identifier: GPL-2.0
/*
 * process related struct kmem cache
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

#ifndef _UPATCH_PROCESS_CACHE_H
#define _UPATCH_PROCESS_CACHE_H

#include <linux/init.h>
#include <linux/slab.h>

extern struct kmem_cache *g_process_cache;
extern struct kmem_cache *g_patch_info_cache;
extern struct kmem_cache *g_jump_entry_cache;

int __init process_cache_init(void);
void __exit process_cache_exit(void);

#endif // _UPATCH_PROCESS_CACHE_H
