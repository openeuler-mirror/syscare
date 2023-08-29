// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
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

struct upatch_conmsg {
    const char *binary;
    const char *patch;
};

#define UPATCH_DEV_NAME "upatch"

#define UPATCH_IOCTL_MAGIC 0xE5

/* when apply: patch information will be recored to the context of the process */
#define UPATCH_ATTACH_PATCH _IOW(UPATCH_IOCTL_MAGIC, 0x7, const struct upatch_conmsg *)

#define UPATCH_REMOVE_PATCH _IOW(UPATCH_IOCTL_MAGIC, 0x8, const char *)

#define UPATCH_ACTIVE_PATCH _IOW(UPATCH_IOCTL_MAGIC, 0x9, const char *)

/* deactive the jmp instruction but do not remove */
#define UPATCH_DEACTIVE_PATCH _IOW(UPATCH_IOCTL_MAGIC, 0xA, const char *)

#define UPATCH_INFO_PATCH _IOW(UPATCH_IOCTL_MAGIC, 0xB, const char *)

#endif /* _UPATCH_IOCTL_H */
