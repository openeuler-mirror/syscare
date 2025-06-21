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

#include "ioctl_dev.h"

#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>
#include <linux/vmalloc.h>
#include <linux/string.h>
#include <linux/module.h>
#include <linux/slab.h>
#include <linux/err.h>

#include <linux/binfmts.h>

#include "patch_entity.h"
#include "patch_manage.h"
#include "util.h"

struct patch_load_request {
    const char *patch_file;
    const char *target_elf;
};

long handle_ioctl(struct file *file, unsigned int code, unsigned long arg);

static const struct file_operations UPATCH_DEV_OPS = {
    .owner          = THIS_MODULE,
    .unlocked_ioctl = handle_ioctl,
};

static struct miscdevice upatch_dev = {
    .minor = MISC_DYNAMIC_MINOR,
    .name  = UPATCH_DEV_NAME,
    .fops  = &UPATCH_DEV_OPS,
    .mode  = UPATCH_DEV_MODE,
};

static char *vmalloc_string_from_user(void __user *addr)
{
    size_t len;
    char *buf;
    int ret;

    if (addr == 0) {
        return ERR_PTR(-EINVAL);
    }

    len = strnlen_user(addr, MAX_ARG_STRLEN);
    if (len > PATH_MAX) {
        return ERR_PTR(-EOVERFLOW);
    }

    buf = vmalloc(len);
    if (!buf) {
        log_err("failed to vmalloc string, len=0x%lx\n", len);
        return ERR_PTR(-ENOMEM);
    }

    ret = copy_from_user(buf, addr, len);
    if (ret) {
        VFREE_CLEAR(buf);
        return ERR_PTR(ret);
    }

    return buf;
}

static int get_load_para_from_user(void __user *user_addr, struct patch_load_request *res)
{
    struct patch_load_request req;
    int error;
    int ret;

    ret = copy_from_user(&req, user_addr, sizeof(struct patch_load_request));
    if (ret) {
        log_err("failed to get target elf path, ret=%d\n", ret);
        return -EINVAL;
    }

    res->target_elf = vmalloc_string_from_user((void __user *)req.target_elf);
    if (IS_ERR(res->target_elf)) {
        error = PTR_ERR(res->target_elf);
        log_err("failed to get target elf path, ret=%d\n", error);
        return error;
    }

    res->patch_file = vmalloc_string_from_user((void __user *)req.patch_file);
    if (IS_ERR(res->patch_file)) {
        error = PTR_ERR(res->patch_file);
        log_err("failed to get patch file path, ret=%d\n", error);
        vfree(res->patch_file);
        return error;
    }

    return 0;
}

static int ioctl_get_patch_status(void __user * user_addr)
{
    int ret;

    char *patch = vmalloc_string_from_user(user_addr);

    if (IS_ERR(patch)) {
        log_err("failed to get patch file path\n");
        return PTR_ERR(patch);
    }

    ret = upatch_status(patch);
    log_debug("patch '%s' is %s\n", patch, patch_status_str(ret));

    vfree(patch);
    return ret;
}

static int ioctl_load_patch(void __user * user_addr)
{
    int ret;
    struct patch_load_request req;

    if (!try_module_get(THIS_MODULE)) {
        log_err("cannot increase '%s' refcnt!", THIS_MODULE->name);
        return -ENODEV;
    }

    ret = get_load_para_from_user(user_addr, &req);
    if (ret) {
        log_err("failed to get patch file path\n");
        module_put(THIS_MODULE);
        return ret;
    }

    ret = upatch_load(req.patch_file, req.target_elf);
    if (ret) {
        log_err("failed to load '%s' for '%s', ret=%d\n",
            req.patch_file, req.target_elf, ret);
        module_put(THIS_MODULE);
    }

    vfree(req.patch_file);
    vfree(req.target_elf);
    return ret;
}

static int ioctl_active_patch(void __user * user_addr)
{
    int ret;
    char *patch = vmalloc_string_from_user(user_addr);

    if (IS_ERR(patch)) {
        log_err("failed to get patch file path\n");
        return PTR_ERR(patch);
    }

    ret = upatch_active(patch);
    if (ret) {
        log_err("failed to active patch '%s', ret=%d\n", patch, ret);
    }

    vfree(patch);
    return ret;
}

static int ioctl_deactive_patch(void __user * user_addr)
{
    int ret;
    char *patch = vmalloc_string_from_user(user_addr);

    if (IS_ERR(patch)) {
        log_err("failed to get patch file path\n");
        return PTR_ERR(patch);
    }

    ret = upatch_deactive(patch);
    if (ret) {
        log_err("failed to deactive patch '%s', ret=%d\n", patch, ret);
    }

    vfree(patch);
    return ret;
}

static int ioctl_remove_patch(void __user * user_addr)
{
    int ret;
    char *patch = vmalloc_string_from_user(user_addr);

    if (IS_ERR(patch)) {
        log_err("failed to get patch file path\n");
        return PTR_ERR(patch);
    }

    ret = upatch_remove(patch);
    if (ret) {
        log_err("failed to remove patch %s, ret=%d\n", patch, ret);
    } else {
        module_put(THIS_MODULE);
    }

    vfree(patch);
    return ret;
}

long handle_ioctl(struct file *file, unsigned int code, unsigned long arg)
{
    unsigned int type = _IOC_TYPE(code);
    unsigned int nr = _IOC_NR(code);
    void __user *argp = (void __user *)arg;

    if (type != UPATCH_MAGIC) {
        log_err("invalid ioctl type 0x%x\n", type);
        return -EINVAL;
    }

    switch (nr) {
        case UPATCH_STATUS:
            return ioctl_get_patch_status(argp);

        case UPATCH_LOAD:
            return ioctl_load_patch(argp);

        case UPATCH_ACTIVE:
            return ioctl_active_patch(argp);

        case UPATCH_DEACTIVE:
            return ioctl_deactive_patch(argp);

        case UPATCH_REMOVE:
            return ioctl_remove_patch(argp);

        default:
            log_err("invalid ioctl nr 0x%x\n", nr);
            return -EINVAL;
    }

    return 0;
}

int __init ioctl_device_init(void)
{
    return misc_register(&upatch_dev);
}

void __exit ioctl_device_exit(void)
{
    misc_deregister(&upatch_dev);
}
