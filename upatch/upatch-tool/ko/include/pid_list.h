// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#ifndef _UPATCH_PID_LIST_H
#define _UPATCH_PID_LIST_H

typedef struct pid_list {
    struct list_head list_head;
    struct mutex list_mutex;
} pid_list_t;

typedef struct pid_list_entry {
    struct list_head list_node;
    pid_t pid;
} pid_list_entry_t;

pid_list_t* alloc_pid_list(void);
void free_pid_list(pid_list_t *list);

pid_list_entry_t* find_pid_list(pid_list_t *list, pid_t pid);
int insert_pid_list(pid_list_t *list, pid_t pid);
void remove_pid_list(pid_list_t *list, pid_t pid);
pid_list_entry_t *get_pid_list_first_entry(pid_list_t *list);
void remove_pid_list_entry(pid_list_t *list, pid_list_entry_t *entry);

#endif
