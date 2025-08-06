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
#define UPATCH_DEV_MODE 0600

#define UPATCH_MAGIC 0xE5

enum upatch_command {
    UPATCH_LOAD = 1,
    UPATCH_ACTIVE,
    UPATCH_DEACTIVE,
    UPATCH_REMOVE,
    UPATCH_STATUS,
};

struct upatch_request_user {
    const char __user *target_elf;
    const char __user *patch_file;
};

struct upatch_request {
    const char *target_elf;
    const char *patch_file;
};

#define _UPATCH_IOCTL(cmd, type) _IOW(UPATCH_MAGIC, cmd, type)

#define UPATCH_LOAD_IOCTL     _UPATCH_IOCTL(UPATCH_LOAD, const struct upatch_request *)
#define UPATCH_ACTIVE_IOCTL   _UPATCH_IOCTL(UPATCH_ACTIVE, const struct upatch_request *)
#define UPATCH_DEACTIVE_IOCTL _UPATCH_IOCTL(UPATCH_DEACTIVE, const struct upatch_request *)
#define UPATCH_REMOVE_IOCTL   _UPATCH_IOCTL(UPATCH_REMOVE, const struct upatch_request *)
#define UPATCH_STATUS_IOCTL   _UPATCH_IOCTL(UPATCH_STATUS, const struct upatch_request *)

int __init ioctl_device_init(void);
void __exit ioctl_device_exit(void);

#endif // _UPATCH_MANAGE_IOCTL_DEV_H
