// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#ifndef _UPATCH_UPROBE_LIST_H
#define _UPATCH_UPROBE_LIST_H

#include <linux/limits.h>

typedef int uprbid_t;

typedef struct uprobe_list {
    struct list_head list_head;
    struct mutex list_mutex;
} uprobe_list_t;

typedef struct uprobe_list_entry {
    uprbid_t id;
    struct list_head list_node;
    struct inode *inode;
    loff_t offset;
    char binary_path[PATH_MAX];
    char patch_path[PATH_MAX];
    pid_t pid;
} uprobe_list_entry_t;

uprobe_list_t* alloc_uprobe_list(void);
void free_uprobe_list(uprobe_list_t *list);

uprobe_list_entry_t* find_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset);
int insert_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset, char *binary_path, char *patch_path);
void remove_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset);

#endif
