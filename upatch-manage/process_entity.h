// SPDX-License-Identifier: GPL-2.0
/*
 * maintain userspace process info if it have loaded a hot patch
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

#ifndef _UPATCH_MANAGE_PROCESS_ENTITY_H
#define _UPATCH_MANAGE_PROCESS_ENTITY_H

#include <linux/types.h>
#include <linux/list.h>
#include <linux/mutex.h>

#include <linux/hashtable.h>

struct pid;
struct task_struct;

struct patch_entity;
struct target_entity;

// we assume one patch will only modify less than 2^4 = 16 old funcs in target
#define FUNC_HASH_BITS 4

struct pc_pair {
    unsigned long old_pc;
    unsigned long new_pc;
    struct hlist_node node;     // maintain pc pair in <old, new> hash table
};

struct patch_info {
    struct patch_entity *patch;
    struct list_head list;
    DECLARE_HASHTABLE(pc_maps, FUNC_HASH_BITS);
};

// target may be loaded into different process
// due to latency of uprobe handle, process may dealy patch loading
struct process_entity {
    struct pid* pid_s;
    struct task_struct* task;
    struct patch_info *active_info;

    // multi-thread may trap and run uprobe_handle, we only need one to load patch
    struct mutex lock;

    // loaded but deactive patch will not free from VMA because the function of deactive patch may in call stack
    // so we have to maintain all patches the process loaded
    // For example we load and active p1, p2, p3, the patches list will be p3->p2->p1
    // when we want to active p2, we just look through this list and active p2 to avoid load p2 again
    struct list_head loaded_patches;    // patch_info list head

    struct list_head list;
};

struct process_entity *get_process(struct target_entity *target);

void free_process(struct process_entity *process);

void free_patch_info(struct patch_info *info);

#endif // _UPATCH_MANAGE_PROCESS_ENTITY_H
