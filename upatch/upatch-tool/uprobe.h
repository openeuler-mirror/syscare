// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
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
