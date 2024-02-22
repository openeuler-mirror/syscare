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
} uprobe_list_entry_t;

uprobe_list_t* alloc_uprobe_list(void);
void free_uprobe_list(uprobe_list_t *list);

uprobe_list_entry_t* find_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset);
int insert_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset, char *binary_path, char *patch_path);
void remove_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset);

#endif
