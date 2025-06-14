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

#include "process_entity.h"

#include <linux/sched/task.h>

#include "target_entity.h"
#include "util.h"

void free_patch_info(struct patch_info *info)
{
    struct pc_pair *pair;
    struct hlist_node *tmp;
    int bkt;

    hash_for_each_safe(info->pc_maps, bkt, tmp, pair, node) {
        hash_del(&pair->node);
        kfree(pair);
    }

    kfree(info);
}

// process may be exited already, free loaded patch info
void free_process(struct process_entity* process)
{
    struct patch_info *info;
    struct patch_info *tmp;
    pid_t pid;

    pid = pid_nr(process->pid_s);

    log_debug("free process tgid %d\n", pid);

    put_pid(process->pid_s);

    mutex_lock(&process->lock);
    list_for_each_entry_safe(info, tmp, &process->loaded_patches, list) {
        list_del(&info->list);
        free_patch_info(info);
    }

    list_del(&process->list);
    mutex_unlock(&process->lock);
    kfree(process);
}

// we use struct pid to reference different process
static struct process_entity *do_get_process_and_free_exit_process(struct target_entity *target)
{
    struct process_entity *process = NULL;
    struct process_entity *tmp = NULL;
    struct process_entity *res = NULL;
    struct pid *pid_s;
    struct task_struct *task;

    pid_s = get_task_pid(current, PIDTYPE_TGID);

    list_for_each_entry_safe(process, tmp, &target->process_head, list) {
        task = get_pid_task(process->pid_s, PIDTYPE_TGID);
        if (!task) {
            // old process is exit, so task is NULL, free it
            free_process(process);
            continue;
        }
        put_task_struct(task);

        if (process->pid_s != pid_s) {
            continue;
        }

        res = process;
        break;
    }

    put_pid(pid_s);
    return res;
}

static struct process_entity *new_process(struct target_entity *target)
{
    struct process_entity *process = kzalloc(sizeof(struct process_entity), GFP_KERNEL);
    if (!process) {
        return NULL;
    }

    log_debug("Create process tgid %d for %s\n", task_tgid_nr(current), target->path);
    process->pid_s = get_task_pid(current, PIDTYPE_TGID);
    process->task = current;
    INIT_LIST_HEAD(&process->loaded_patches);
    mutex_init(&process->lock);
    list_add(&process->list, &target->process_head);
    return process;
}

struct process_entity *get_process(struct target_entity *target)
{
    struct process_entity *process = NULL;

    mutex_lock(&target->process_lock);
    process = do_get_process_and_free_exit_process(target);
    if (!process) {
        process = new_process(target);
        if (!process) {
            log_err("cannot alloc process tgid %d, for target %s\n",
                task_tgid_nr(current), target->path);
            goto out;
        }
    }

out:
    mutex_unlock(&target->process_lock);
    return process;
}
