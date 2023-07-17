// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2023 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_IOCTL_H
#define _UPATCH_IOCTL_H

/* ATTENTION: This head file is exported to userspace */
#include <linux/ioctl.h>
#include <linux/types.h>

struct upatch_hijack_msg {
    unsigned long compiler_ino;
    unsigned long hijacker_ino;
    const char *driver_name;
    const char *hijacker_name;
};

#define UPATCH_BUILD_DEV_NAME   "upatch-build"

#define UPATCH_IOCTL_MAGIC 0xE5

/* used for upatch-build */
#define UPATCH_REGISTER_ENTRY _IOW(UPATCH_IOCTL_MAGIC, 0x1, const struct upatch_hijack_msg *)

#define UPATCH_UNREGISTER_ENTRY _IOW(UPATCH_IOCTL_MAGIC, 0x2, const struct upatch_hijack_msg *)

#endif /* _UPATCH_IOCTL_H */
