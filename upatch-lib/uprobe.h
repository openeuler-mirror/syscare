// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-lib
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

#ifndef _UPATCH_UPROBE_H
#define _UPATCH_UPROBE_H

#include "monitor_list.h"
//typedef uprbid_t;

struct upatch_uprobe {
    uprobe_list_t *ulist;
    pid_list_t *plist;
};

extern monitor_list_t *monitor_list;
int monitor_list_init(void);
void monitor_list_destroy(void);
int upatch_monitor_register(monitor_list_t *mlist, pid_t monitor_pid);
int __upatch_uprobe_deregister(struct inode *inode, loff_t offset);
void upatch_monitor_deregister(void __user *param, monitor_list_t *mlist);

int uprobe_init(monitor_list_t *mlist);
void uprobe_destroy(monitor_list_t *mlist);

int upatch_uprobe_register(monitor_list_t *mlist, struct inode *inode, loff_t offset, char *binary_path, char *patch_path);
int upatch_uprobe_deregister(monitor_list_t *mlist, struct inode *inode, loff_t offset, pid_t monitor_pid);

#endif
