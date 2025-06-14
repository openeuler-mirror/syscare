// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_MANAGE_IOCTL_DEV_H
#define _UPATCH_MANAGE_IOCTL_DEV_H

#include <linux/init.h>

#define UPATCH_DEV_NAME "upatch_manage"
#define UPATCH_DEV_PATH "/dev/upatch_manage"
#define UPATCH_DEV_MODE 0600

#define UPATCH_MAGIC 0xE5

enum {
    UPATCH_LOAD = 1,
    UPATCH_ACTIVE,
    UPATCH_DEACTIVE,
    UPATCH_REMOVE,
    UPATCH_STATUS,
};

#define _UPATCH_IOCTL(cmd, type) _IOW(UPATCH_MAGIC, cmd, type)

#define UPATCH_LOAD_IOCTL     _UPATCH_IOCTL(UPATCH_LOAD, const struct load_request *)
#define UPATCH_ACTIVE_IOCTL   _UPATCH_IOCTL(UPATCH_ACTIVE, const char *)
#define UPATCH_DEACTIVE_IOCTL _UPATCH_IOCTL(UPATCH_DEACTIVE, const char *)
#define UPATCH_REMOVE_IOCTL   _UPATCH_IOCTL(UPATCH_REMOVE, const char *)
#define UPATCH_STATUS_IOCTL   _UPATCH_IOCTL(UPATCH_STATUS, const char *)

int __init ioctl_device_init(void);
void __exit ioctl_device_exit(void);

#endif // _UPATCH_MANAGE_IOCTL_DEV_H
