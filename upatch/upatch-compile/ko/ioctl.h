// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-hijacker kernel module
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

#ifndef _UPATCH_HIJACKER_KO_IOCTL_H
#define _UPATCH_HIJACKER_KO_IOCTL_H

#include <linux/types.h>
#include <linux/limits.h>

#define UPATCH_HIJACKER_DEV_NAME "upatch-hijacker"
#define UPATCH_HIJACKER_DEV_MODE 0600

#define UPATCH_HIJACKER_IOC_MAGIC 0xE5
#define UPATCH_HIJACKER_ENABLE _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x1, \
    upatch_enable_request_t)
#define UPATCH_HIJACKER_DISABLE _IO(UPATCH_HIJACKER_IOC_MAGIC, 0x2)
#define UPATCH_HIJACKER_REGISTER _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x3, \
    upatch_register_request_t)
#define UPATCH_HIJACKER_UNREGISTER _IOW(UPATCH_HIJACKER_IOC_MAGIC, 0x4, \
    upatch_register_request_t)

typedef struct {
    char path[PATH_MAX];
    loff_t offset;
} upatch_enable_request_t;

typedef struct {
    char exec_path[PATH_MAX];
    char jump_path[PATH_MAX];
} upatch_register_request_t;

struct file;

int ioctl_init(void);
void ioctl_exit(void);
long handle_ioctl(struct file *file, unsigned int cmd, unsigned long arg);

#endif /* _UPATCH_HIJACKER_KO_IOCTL_H */
