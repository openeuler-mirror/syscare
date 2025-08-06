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

static char *read_patch_requrest_param(const char __user *uaddr)
{
    long len;
    char *buf;

    if (unlikely(!uaddr)) {
        return ERR_PTR(-EINVAL);
    }

    len = strnlen_user(uaddr, PATH_MAX);
    if (unlikely(len <= 0)) {
        return ERR_PTR(-EINVAL);
    }
    if (unlikely(len >= PATH_MAX)) {
        return ERR_PTR(-ENAMETOOLONG);
    }

    buf = vmalloc(len + 1);
    if (unlikely(!buf)) {
        return ERR_PTR(-ENOMEM);
    }

    if (unlikely(copy_from_user(buf, uaddr, len))) {
        vfree(buf);
        return ERR_PTR(-EFAULT);
    }
    buf[len] = '\0';

    return buf;
}

static inline void free_patch_request(struct upatch_request *request)
{
    vfree(request->target_elf);
    vfree(request->patch_file);
}

static int read_patch_request(struct upatch_request *kreq, void __user *uaddr)
{
    struct upatch_request_user ureq = { 0 };
    int ret = 0;

    kreq->target_elf = NULL;
    kreq->patch_file = NULL;

    if (unlikely(copy_from_user(&ureq, uaddr, sizeof(ureq)))) {
        return -EFAULT;
    }

    kreq->target_elf = read_patch_requrest_param(ureq.target_elf);
    if (unlikely(IS_ERR(kreq->target_elf))) {
        ret = PTR_ERR(kreq->target_elf);
        kreq->target_elf = NULL;
        goto out_err;
    }

    kreq->patch_file = read_patch_requrest_param(ureq.patch_file);
    if (unlikely(IS_ERR(kreq->patch_file))) {
        ret = PTR_ERR(kreq->patch_file);
        kreq->patch_file = NULL;
        goto out_err;
    }

    return 0;

out_err:
    free_patch_request(kreq);
    return ret;
}

long handle_ioctl(struct file *file, unsigned int code, unsigned long arg)
{
    unsigned int type = _IOC_TYPE(code);
    unsigned int nr = _IOC_NR(code);
    void __user *msg = (void __user *)arg;

    struct upatch_request req;
    int ret = 0;

    if (unlikely(type != UPATCH_MAGIC || !msg)) {
        log_err("invalid ioctl message\n");
        return -EINVAL;
    }

    ret = read_patch_request(&req, msg);
    if (unlikely(ret)) {
        log_err("failed to read patch requrest, ret=%d\n", ret);
        return ret;
    }

    switch (nr) {
        case UPATCH_LOAD:
            ret = upatch_load(req.target_elf, req.patch_file);
            break;
        case UPATCH_REMOVE:
            ret = upatch_remove(req.target_elf, req.patch_file);
            break;
        case UPATCH_ACTIVE:
            ret = upatch_active(req.target_elf, req.patch_file);
            break;
        case UPATCH_DEACTIVE:
            ret = upatch_deactive(req.target_elf, req.patch_file);
            break;
        case UPATCH_STATUS:
            ret = upatch_status(req.target_elf, req.patch_file);
            break;
        default:
            log_err("invalid ioctl nr 0x%x\n", nr);
            ret = -EINVAL;
            break;
    }

    free_patch_request(&req);
    return ret;
}

int __init ioctl_device_init(void)
{
    return misc_register(&upatch_dev);
}

void __exit ioctl_device_exit(void)
{
    misc_deregister(&upatch_dev);
}
