// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
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
    struct map *hijacker_map;
};

static bool find_hijacker_context(const struct context *context,
    const struct pid_namespace *ns);
static void free_hijacker_context(struct context *context);

static const struct map_ops HIJACK_MAP_OPS = {
    .find_value = (find_value_fn)find_hijacker_record,
    .free_value = (free_value_fn)free_hijacker_record,
};
static const struct map_ops CONTEXT_MAP_OPS = {
    .find_value = (find_value_fn)find_hijacker_context,
    .free_value = (free_value_fn)free_hijacker_context,
};

static const size_t MAX_CONTEXT_NUM = 1024;
static const size_t HIJACKER_PER_CONTEXT = 16;

static struct map *g_context_map = NULL;

/* Context private interface */
static int create_hijacker_context(struct context **context,
    struct pid_namespace *ns, const char *path, loff_t offset)
{
    struct context *new_context = NULL;
    struct uprobe_record *uprobe = NULL;
    struct map *hijacker_map = NULL;
    int ret = 0;

    new_context = kzalloc(sizeof(struct context), GFP_KERNEL);
    if (new_context == NULL) {
        pr_err("failed to alloc context\n");
        return -ENOMEM;
    }

    ret = new_map(&hijacker_map, HIJACKER_PER_CONTEXT, &HIJACK_MAP_OPS);
    if (ret != 0) {
        pr_err("failed to create hijacker map, ret=%d\n", ret);
        kfree(new_context);
        return ret;
    }

    ret = new_uprobe_record(&uprobe, handle_uprobe, path, offset);
    if (ret != 0) {
        pr_err("failed to create uprobe record, ret=%d\n", ret);
        free_map(hijacker_map);
        kfree(new_context);
        return ret;
    }

    ret = uprobe_register(uprobe->inode, uprobe->offset, uprobe->uc);
    if (ret != 0) {
        pr_err("failed to register uprobe, inode=%lu, offset=0x%llx, ret=%d\n",
            uprobe->inode->i_ino, uprobe->offset, ret);
        free_uprobe_record(uprobe);
        free_map(hijacker_map);
        kfree(new_context);
        return ret;
    }

    new_context->ns = get_pid_ns(ns);
    new_context->uprobe = uprobe;
    new_context->hijacker_map = hijacker_map;

    *context = new_context;
    return 0;
}

static void free_hijacker_context(struct context *context)
{
    if (context == NULL) {
        return;
    }

    uprobe_unregister(context->uprobe->inode, context->uprobe->offset,
        context->uprobe->uc);

    put_pid_ns(context->ns);
    free_uprobe_record(context->uprobe);
    free_map(context->hijacker_map);
    kfree(context);
}

static bool find_hijacker_context(const struct context *context,
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

int build_hijacker_context(const char *path, loff_t offset)
{
    struct pid_namespace *ns = task_active_pid_ns(current);
    struct context *context = NULL;
    int ret = 0;

    if ((path == NULL) || (offset == 0)) {
        return -EINVAL;
    }

    ret = create_hijacker_context(&context, ns, path, offset);
    if (ret != 0) {
        pr_err("failed to create hijacker context, ret=%d\n", ret);
        return ret;
    }

    pr_debug("hijacker context, addr=0x%lx\n", (unsigned long)context);
    ret = map_insert(g_context_map, context);
    if (ret != 0) {
        pr_err("failed to register hijacker context, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

void destroy_hijacker_context(void)
{
    pr_debug("destroy hijacker context\n");
    map_remove(g_context_map, task_active_pid_ns(current));
}

size_t hijacker_context_count(void)
{
    return map_size(g_context_map);
}

struct map *get_hijacker_map(void)
{
    struct pid_namespace *ns = task_active_pid_ns(current);
    struct context *context = (struct context *)map_get(g_context_map, ns);

    return (context != NULL) ? context->hijacker_map : NULL;
}
