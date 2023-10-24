// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#ifndef _UPATCH_IOCTL_H
#define _UPATCH_IOCTL_H

#define UPATCH_IOCTL_MAGIC 0xE5

typedef struct elf_request {
    char elf_path[PATH_MAX];
    loff_t offset;
    pid_t monitor_pid;
    char patch_path[PATH_MAX];
} elf_request_t;

#define UPATCH_UPROBE_INIT		_IO(UPATCH_IOCTL_MAGIC, 0x5)
#define UPATCH_UPROBE_DESTROY		_IO(UPATCH_IOCTL_MAGIC, 0x6)
#define UPATCH_REGISTER_ELF		_IOW(UPATCH_IOCTL_MAGIC, 0x7, const elf_request_t *)
#define UPATCH_DEREGISTER_ELF		_IOW(UPATCH_IOCTL_MAGIC, 0x8, const elf_request_t *)
#define UPATCH_GET_PID			_IOW(UPATCH_IOCTL_MAGIC, 0x9, struct upatch_pid *)
#define UPATCH_REGISTER_MONITOR		_IO(UPATCH_IOCTL_MAGIC, 0x10)
#define UPATCH_DEREGISTER_MONITOR	_IO(UPATCH_IOCTL_MAGIC, 0x11)

long handle_ioctl(struct file *filp, unsigned int cmd, unsigned long arg);

#endif /* _UPATCH_IOCTL_H */
