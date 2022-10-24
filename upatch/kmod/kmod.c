// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/kernel.h>
#include <linux/module.h>

#include "kmod.h"
#include "compiler.h"

struct kobject *upatch_kobj;

static int __init upatch_init(void)
{
    int ret;

    upatch_kobj = kobject_create_and_add("upatch", kernel_kobj);
    if (!upatch_kobj)
        return -ENOMEM;

    ret = compiler_hack_init();
    if (ret < 0)
        goto kobj_out;

    goto out;
kobj_out:
    kobject_put(upatch_kobj);
out:
    return ret;
}

static void __exit upatch_exit(void)
{
    compiler_hack_exit();
    kobject_put(upatch_kobj);
}

module_init(upatch_init);
module_exit(upatch_exit);

MODULE_AUTHOR("Longjun Luo (luolongjuna@gmail.com)");
MODULE_DESCRIPTION("kernel module for upatch(live-patch in userspace)");
MODULE_LICENSE("GPL");