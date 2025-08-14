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
#include <linux/mutex.h>
#include <linux/sched.h>
#include <linux/kref.h>
#include <linux/spinlock.h>

#include <linux/hashtable.h>

struct pid;
struct task_struct;

struct patch_context;
struct patch_entity;
struct target_entity;

#define PATCH_FUNC_HASH_BITS 6  // Single patch would have less than 2^6 = 64 funcs

struct patch_jump_entry {
    struct hlist_node node;

    unsigned long old_addr;
    unsigned long new_addr;
    unsigned long new_end;
};

struct patch_info {
    struct patch_entity *patch;
    struct list_head node;

    unsigned long text_addr;
    size_t text_len;

    unsigned long rodata_addr;
    size_t rodata_len;

    unsigned long jump_min_addr;
    unsigned long jump_max_addr;

    DECLARE_HASHTABLE(jump_table, PATCH_FUNC_HASH_BITS);
};

// target may be loaded into different process
// due to latency of uprobe handle, process may dealy patch loading
struct process_entity {
    struct task_struct *task;       // underlying task struct
    pid_t tgid;

    spinlock_t thread_lock;         // thread lock

    struct hlist_node node;         // hash table node
    struct list_head pending_node;  // pending list node

    struct list_head patch_list;    // all actived patches
    struct patch_info *patch_info;  // current actived patch info

    struct kref kref;
};

/**
 * @brief Create and initialize a new process entity for a given task.
 * @param task The kernel task_struct to be wrapped by the new entity.
 *             This function will take its own reference to the task via
 *             get_task_struct().
 *
 * @return On success, returns a pointer to the allocated process_entity with
 *         its reference count initialized to 1.
 *         On memory allocation failure, returns an ERR_PTR (e.g., ERR_PTR(-ENOMEM)).
 *
 * The caller owns the returned reference and is responsible for releasing it
 * using put_process() when it is no longer needed.
 */
struct process_entity *new_process(struct task_struct *task);

/**
 * @brief Release process resources when refcount reaches zero
 * @param kref: Reference counter
 *
 * Called automatically by kref_put().
 * Frees all process resources and disassociates from target.
 */
void release_process(struct kref *kref);

/**
 * @brief Acquire a reference to a process entity
 * @param process: Process entity pointer
 * @return Process entity with incremented refcount, or NULL if input is NULL
 *
 * Caller must balance with put_process().
 */
static inline struct process_entity *get_process(struct process_entity *process)
{
    if (unlikely(!process)) {
        return NULL;
    }

    kref_get(&process->kref);
    return process;
}

/**
 * @brief Release a process entity reference
 * @param process: Process entity pointer
 *
 * Decrements refcount and triggers release_process() when reaching zero.
 * Safe to call with NULL.
 */
static inline void put_process(struct process_entity *process)
{
    if (unlikely(!process)) {
        return;
    }

    kref_put(&process->kref, release_process);
}

/**
 * @brief Check if a process entity's underlying task is still alive.
 * @param process: The process entity to check.
 * @return Returns true if the task is considered alive by the kernel,
 *         false otherwise.
 *
 * Safe to call with NULL; it will be treated as not alive.
 */
static inline bool process_is_alive(struct process_entity *process)
{
    if (unlikely(!process || !process->task)) {
        return false;
    }

    return pid_alive(process->task);
}

/**
 * @brief Switch and get process actived patch to specific one
 * @param process: Process entity (must not NULL)
 * @param patch: Patch entity (must not NULL)
 * @return Patch info pointer, NULL if not found
 *
 * Caller must hold thread lock.
 */
struct patch_info *process_switch_and_get_patch(struct process_entity *process, struct patch_entity *patch);

/**
 * @brief Find function jump address in the process
 * @param process: Process entity
 * @param addr: Current function address (usually pc register)
 * @return Jump address if found, 0 otherwise
 *
 * Caller must hold thread lock.
 */
unsigned long process_get_jump_addr(struct process_entity *process, unsigned long addr);

/**
 * @brief Load a patch to the process
 * @param process: Process entity
 * @param patch: Patch entity being applied
 * @param ctx: Patch context
 * @return 0 on success, negative error code on failure
 *
 * Caller must hold thread lock.
 */
int process_load_patch(struct process_entity *process, struct patch_entity *patch, struct patch_context *ctx);

/**
 * @brief Remove patch is on the process
 * @param process: Process entity
 * @param patch: Patch entity to remove
 *
 * Caller must hold thread lock.
 */
void process_remove_patch(struct process_entity *process, struct patch_entity *patch);

/**
 * @brief Verify patch is not on process stack
 * @param process: Process entity
 * @param patch: Patch entity to verify
 * @return 0 if safe to modify, -EBUSY if patch is on the stack
 *
 * Caller must hold thread lock.
 */
int process_check_patch_on_stack(struct process_entity *process, struct patch_entity *patch);

#endif // _UPATCH_MANAGE_PROCESS_ENTITY_H
