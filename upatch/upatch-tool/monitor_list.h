// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
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
