// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-helper kernel module
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

#include "cache.h"

#include <linux/slab.h>

#include "log.h"

static const char *CACHE_SLAB_NAME = "upatch_helper";

static struct kmem_cache *g_path_cache = NULL;

int cache_init(void)
{
    g_path_cache = kmem_cache_create_usercopy(CACHE_SLAB_NAME,
        PATH_MAX, 0, SLAB_MEM_SPREAD | SLAB_ACCOUNT | SLAB_RECLAIM_ACCOUNT,
        0, PATH_MAX, NULL);
    if (g_path_cache == NULL) {
        pr_err("failed to create slab '%s'\n", CACHE_SLAB_NAME);
        return -ENOMEM;
    }
    return 0;
}

void cache_exit(void)
{
    kmem_cache_destroy(g_path_cache);
}

char *path_buf_alloc(void)
{
    return kmem_cache_alloc(g_path_cache, GFP_KERNEL);
}

void path_buf_free(char *buff)
{
    if (buff == NULL) {
        return;
    }
    kmem_cache_free(g_path_cache, buff);
}
