// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#include "cache.h"

#include <linux/slab.h>

#include "log.h"

static const char *CACHE_SLAB_NAME = "upatch_hijacker";

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
