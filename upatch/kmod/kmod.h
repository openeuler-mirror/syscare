// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_KMOD_H
#define _UPATCH_KMOD_H

#include <linux/ioctl.h>
#include <linux/types.h>

#include "upatch-ioctl.h"

#define UPATCH_KPROBE_NUM       1
#define UPATCH_KPROBE_MPROTECT  0
extern struct kprobe *upatch_kprobes[UPATCH_KPROBE_NUM];

#endif /* _UPATCH_KMOD_H */
