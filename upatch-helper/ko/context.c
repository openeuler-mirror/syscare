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

#include "context.h"

#include <linux/fs.h>
#include <linux/pid_namespace.h>
#include <linux/slab.h>
#include <linux/uprobes.h>

#include "log.h"
#include "map.h"
#include "records.h"
#include "uprobe.h"
#include "utils.h"

struct context {
    struct pid_namespace *ns;
    struct uprobe_record *uprobe;
    struct map *helper_map;
};

static bool find_helper_context(const struct context *context,
    const struct pid_namespace *ns);
static void free_helper_context(struct context *context);

static const struct map_ops HELPER_MAP_OPS = {
    .find_value = (find_value_fn)find_helper_record,
    .free_value = (free_value_fn)free_helper_record,
};
static const struct map_ops CONTEXT_MAP_OPS = {
    .find_value = (find_value_fn)find_helper_context,
    .free_value = (free_value_fn)free_helper_context,
};

static const size_t MAX_CONTEXT_NUM = 1024;
static const size_t HELPER_PER_CONTEXT = 16;

static struct map *g_context_map = NULL;

/* Context private interface */
static int create_helper_context(struct context **context,
    struct pid_namespace *ns, const char *path, loff_t offset)
{
    struct context *new_context = NULL;
    struct uprobe_record *uprobe = NULL;
    struct map *helper_map = NULL;
    int ret = 0;

    new_context = kzalloc(sizeof(struct context), GFP_KERNEL);
    if (new_context == NULL) {
        pr_err("failed to alloc context\n");
        return -ENOMEM;
    }

    ret = new_map(&helper_map, HELPER_PER_CONTEXT, &HELPER_MAP_OPS);
    if (ret != 0) {
        pr_err("failed to create helper map, ret=%d\n", ret);
        kfree(new_context);
        return ret;
    }

    ret = new_uprobe_record(&uprobe, handle_uprobe, path, offset);
    if (ret != 0) {
        pr_err("failed to create uprobe record, ret=%d\n", ret);
        free_map(helper_map);
        kfree(new_context);
        return ret;
    }

    ret = uprobe_register(uprobe->inode, uprobe->offset, uprobe->uc);
    if (ret != 0) {
        pr_err("failed to register uprobe, inode=%lu, offset=0x%llx, ret=%d\n",
            uprobe->inode->i_ino, uprobe->offset, ret);
        free_uprobe_record(uprobe);
        free_map(helper_map);
        kfree(new_context);
        return ret;
    }

    new_context->ns = get_pid_ns(ns);
    new_context->uprobe = uprobe;
    new_context->helper_map = helper_map;

    *context = new_context;
    return 0;
}

static void free_helper_context(struct context *context)
{
    if (context == NULL) {
        return;
    }

    uprobe_unregister(context->uprobe->inode, context->uprobe->offset,
        context->uprobe->uc);

    put_pid_ns(context->ns);
    free_uprobe_record(context->uprobe);
    free_map(context->helper_map);
    kfree(context);
}

static bool find_helper_context(const struct context *context,
    const struct pid_namespace *ns)
{
    return ns_equal(context->ns, ns);
}

/* Context public interface */
int context_init(void)
{
    int ret = 0;

    ret = new_map(&g_context_map, MAX_CONTEXT_NUM, &CONTEXT_MAP_OPS);
    if (ret != 0) {
        pr_err("failed to create context map, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

void context_exit(void)
{
    free_map(g_context_map);
}

int build_helper_context(const char *path, loff_t offset)
{
    struct pid_namespace *ns = task_active_pid_ns(current);
    struct context *context = NULL;
    int ret = 0;

    if ((path == NULL) || (offset == 0)) {
        return -EINVAL;
    }

    ret = create_helper_context(&context, ns, path, offset);
    if (ret != 0) {
        pr_err("failed to create helper context, ret=%d\n", ret);
        return ret;
    }

    pr_debug("helper context, addr=0x%lx\n", (unsigned long)context);
    ret = map_insert(g_context_map, context);
    if (ret != 0) {
        pr_err("failed to register helper context, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

void destroy_helper_context(void)
{
    pr_debug("destroy helper context\n");
    map_remove(g_context_map, task_active_pid_ns(current));
}

size_t helper_context_count(void)
{
    return map_size(g_context_map);
}

struct map *get_helper_map(void)
{
    struct pid_namespace *ns = task_active_pid_ns(current);
    struct context *context = (struct context *)map_get(g_context_map, ns);

    return (context != NULL) ? context->helper_map : NULL;
}
