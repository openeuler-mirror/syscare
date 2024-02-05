// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#include <linux/module.h>
#include <linux/types.h>

#include "log.h"
#include "cache.h"
#include "context.h"
#include "ioctl.h"

static int __init upatch_hijacker_init(void)
{
    int ret = 0;

    ret = context_init();
    if (ret != 0) {
        pr_err("failed to init context, ret=%d\n", ret);
        return ret;
    }

    ret = cache_init();
    if (ret != 0) {
        pr_err("failed to init cache, ret=%d\n", ret);
        return ret;
    }

    ret = ioctl_init();
    if (ret != 0) {
        pr_err("failed to init ioctl, ret=%d\n", ret);
        return ret;
    }

    pr_info("%s %s initialized\n", THIS_MODULE->name, THIS_MODULE->version);
    return 0;
}

static void __exit upatch_hijacker_exit(void)
{
    ioctl_exit();
    cache_exit();
    context_exit();
}

module_init(upatch_hijacker_init);
module_exit(upatch_hijacker_exit);

MODULE_AUTHOR("renoseven (dev@renoseven.net)");
MODULE_DESCRIPTION("upatch compiler hijacker");
MODULE_LICENSE("GPL");
MODULE_VERSION(BUILD_VERSION);
