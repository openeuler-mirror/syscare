// SPDX-License-Identifier: GPL-2.0
/*
 * upatch_manage kernel module
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

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/init.h>

#include "kernel_compat.h"
#include "ioctl_dev.h"
#include "patch_entity.h"
#include "target_entity.h"
#include "util.h"

#ifndef MODNAME
#define MODNAME "upatch_manage"
#endif

#ifndef MODVER
#define MODVER "devel"
#endif

static int __init upatch_module_init(void)
{
    int ret;

    ret = kernel_compat_init();
    if (ret) {
        log_err("failed to initialize kernel compat layer, ret=%d\n", ret);
        return ret;
    }

    ret = ioctl_device_init();
    if (ret) {
        log_err("failed to initialize ioctl device, ret=%d\n", ret);
        return ret;
    }

    log_info("%s %s initialized\n", MODNAME, MODVER);
    return 0;
}

/*
 * when call load_patch(), we call module_get(). When call remove_patch(), we call module_put().
 * This ensures that the module cannot be rmmod while patches are active or deactive.
 * When upatch_exit() is called, module refcnt should be 0, meaning all patches have been removed.
 * remove_patch() frees patch_entity, and if all patch_entity are free, target_entity will be freed too.
 * At this point, there should be no patches or targets left. If any are found, we must print error
 */
static void __exit upatch_module_exit(void)
{
    verify_patch_empty_on_exit();
    verify_target_empty_on_exit();

    kernel_compat_exit();
    ioctl_device_exit();

    log_info("%s %s exited\n", MODNAME, MODVER);
}

module_init(upatch_module_init);
module_exit(upatch_module_exit);

MODULE_AUTHOR("Longjun Luo (luolongjuna@gmail.com)");
MODULE_AUTHOR("Zongwu Li (lzw32321226@163.com)");
MODULE_AUTHOR("renoseven (dev@renoseven.net)");
MODULE_DESCRIPTION("syscare user patch management");
MODULE_LICENSE("GPL");
MODULE_VERSION(MODVER);
