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
