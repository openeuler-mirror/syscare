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

#ifndef _UPATCH_MONITOR_LIST_H
#define _UPATCH_MONITOR_LIST_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/mutex.h>

#include "uprobe_list.h"
#include "pid_list.h"

typedef struct upatch_monitor_list {
    struct list_head list_head;
    struct mutex list_mutex;
} monitor_list_t;

typedef struct monitor_list_entry {
    pid_t monitor_pid;
    struct list_head list_node;
    uprobe_list_t *uprobe_list;
    pid_list_t *pid_list;
} monitor_list_entry_t;

monitor_list_t* alloc_monitor_list(void);
void free_monitor_list(monitor_list_t *list);

monitor_list_entry_t* alloc_monitor_list_entry(pid_t monitor_pid);
void free_monitor_list_entry(monitor_list_entry_t *entry);

monitor_list_entry_t* find_monitor_list(monitor_list_t *list, pid_t monitor_pid);
int insert_monitor_list(monitor_list_t *list, pid_t monitor_pid);
void remove_monitor_list(monitor_list_t *list, pid_t monitor_pid);

#endif
