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

#include "process_cache.h"

#include "process_entity.h"

struct kmem_cache *g_process_cache;
struct kmem_cache *g_patch_info_cache;
struct kmem_cache *g_jump_entry_cache;

int __init process_cache_init(void)
{
    g_process_cache = kmem_cache_create(
        "upatch_process_entity",
        sizeof(struct process_entity),
        0,
        SLAB_HWCACHE_ALIGN,
        NULL
    );
    if (unlikely(!g_process_cache)) {
        pr_err("failed to create process entity cache\n");
        return -ENOMEM;
    }

    g_patch_info_cache = kmem_cache_create(
        "upatch_patch_info",
        sizeof(struct patch_info),
        0,
        SLAB_HWCACHE_ALIGN,
        NULL
    );
    if (unlikely(!g_patch_info_cache)) {
        pr_err("failed to create patch info cache\n");
        return -ENOMEM;
    }

    g_jump_entry_cache = kmem_cache_create(
        "upatch_jump_entry",
        sizeof(struct patch_jump_entry),
        0,
        SLAB_HWCACHE_ALIGN,
        NULL
    );
    if (unlikely(!g_jump_entry_cache)) {
        pr_err("failed to create patch jump entry cache\n");
        return -ENOMEM;
    }

    return 0;
}

void __exit process_cache_exit(void)
{
    kmem_cache_destroy(g_process_cache);
    kmem_cache_destroy(g_patch_info_cache);
    kmem_cache_destroy(g_jump_entry_cache);
}
