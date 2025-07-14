// SPDX-License-Identifier: GPL-2.0
/*
 * provide kload kactive kdeactive kremove API to manage patch
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

#include "patch_manage.h"

#include <linux/mm.h>
#include <linux/file.h>

#include <linux/hashtable.h>
#include <linux/rwsem.h>

#include "target_entity.h"
#include "process_entity.h"
#include "patch_entity.h"
#include "patch_load.h"
#include "util.h"

/*
 * =====================================================================
 *                        LOCKING HIERARCHY
 * =====================================================================
 * To prevent deadlocks, the following lock acquisition order MUST be
 * strictly followed throughout the entire module:
 *
 * 1. g_global_table_rwsem
 * 2. target_entity->action_rwsem
 * 3. patch_entity->action_rwsem
 *
 * Any deviation from this order will lead to deadlocks.
 * =====================================================================
 */

#define UPROBE_RUN_OLD_FUNC 0

#ifndef UPROBE_ALTER_PC
#define UPROBE_ALTER_PC 2
#endif

#define PATCH_TABLE_HASH_BITS 4
#define TARGET_TABLE_HASH_BITS 4

static DEFINE_HASHTABLE(g_patch_table, PATCH_TABLE_HASH_BITS);
static DEFINE_HASHTABLE(g_target_table, TARGET_TABLE_HASH_BITS);

static DECLARE_RWSEM(g_global_table_rwsem);

static int upatch_uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs);

static struct uprobe_consumer g_upatch_consumer = {
    .handler = upatch_uprobe_handler,
};

/* GLOBAL ENTITY HASH TABLE */
static struct patch_entity *find_patch_by_inode(struct inode *inode)
{
    struct patch_entity *patch;

    hash_for_each_possible(g_patch_table, patch, table_node, inode->i_ino) {
        if (patch->meta.inode == inode) {
            return patch;
        }
    }

    return NULL;
}

struct patch_entity *find_patch(const char *path)
{
    struct patch_entity *patch;
    struct inode *inode;

    inode = get_path_inode(path);
    if (!inode) {
        log_err("failed to get '%s' inode\n", path);
        return NULL;
    }

    patch = find_patch_by_inode(inode);

    iput(inode);
    return patch;
}

static struct target_entity *find_target_by_inode(struct inode *inode)
{
    struct target_entity *target;

    hash_for_each_possible(g_target_table, target, table_node, inode->i_ino) {
        if (target->meta.inode == inode) {
            return target;
        }
    }

    return NULL;
}

static struct target_entity *find_target(const char *path)
{
    struct target_entity *target;
    struct inode *inode;

    inode = get_path_inode(path);
    if (!inode) {
        log_err("failed to get '%s' inode\n", path);
        return NULL;
    }

    target = find_target_by_inode(inode);

    iput(inode);
    return target;
}

/* UPROBE IMPLEMENTATION */
static struct inode *find_vma_file_inode(unsigned long pc, unsigned long *vma_start)
{
    struct mm_struct *mm = current->mm;
    struct vm_area_struct *vma;

    struct file *file = NULL;
    struct inode *inode = NULL;

    mmap_read_lock(mm);

    vma = find_vma(mm, pc);
    if (likely(vma && vma->vm_file)) {
        *vma_start = vma->vm_start;
        file = vma->vm_file;
        get_file(file); // get_file allows NULL pointer
    }

    mmap_read_unlock(mm);

    if (unlikely(!file)) {
        log_err("cannot find vma file on pc 0x%lx\n", pc);
        goto out;
    }

    inode = igrab(file_inode(file));
    if (unlikely(!inode)) {
        log_err("cannot find vma file inode on pc 0x%lx\n", pc);
        goto out;
    }

out:
    if (likely(file)) {
        fput(file);
    }
    return inode;
}

static int jump_to_new_pc(struct pt_regs *regs, const struct patch_info *patch, unsigned long pc)
{
    struct pc_pair *pair;
    struct pc_pair *found_pair = NULL;

    hash_for_each_possible(patch->pc_maps, pair, node, pc) {
        if (pair->old_pc == pc) {
            found_pair = pair;
            break;
        }
    }

    if (unlikely(!found_pair)) {
        log_err("cannot find new pc for 0x%lx\n", pc);
        return UPROBE_RUN_OLD_FUNC;
    }

    log_debug("jump from 0x%lx -> 0x%lx\n", pc, found_pair->new_pc);
    instruction_pointer_set(regs, found_pair->new_pc);

    return UPROBE_ALTER_PC;
}

static void free_exited_process(struct list_head *process_list)
{
    struct process_entity *process;
    struct process_entity *tmp;

    if (unlikely(!process_list)) {
        return;
    }

    list_for_each_entry_safe(process, tmp, process_list, process_node) {
        list_del_init(&process->process_node);
        free_process(process);
    }
}

static int upatch_uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    const char *name = current->comm;
    pid_t pid = task_pid_nr(current);
    pid_t tgid = task_tgid_nr(current);
    unsigned long pc = instruction_pointer(regs);

    struct inode *inode;
    unsigned long vma_start;

    struct target_entity *target;
    struct patch_entity *latest_patch;

    struct list_head exited_proc_list = LIST_HEAD_INIT(exited_proc_list);
    struct process_entity *process;
    struct patch_info *loaded_patch;

    int ret = UPROBE_RUN_OLD_FUNC;

    log_debug("upatch handler triggered on process '%s' (pid=%d, tgid=%d, pc=0x%lx)\n", name, pid, tgid, pc);

    /* find vma file and inode out of the lock */
    inode = find_vma_file_inode(pc, &vma_start);
    if (unlikely(!inode)) {
        log_err("cannot find vma file inode in '%s'\n", name);
        return UPROBE_RUN_OLD_FUNC;
    }

    down_read(&g_global_table_rwsem);

    /* step 1. find target entity by vma file inode */
    target = find_target_by_inode(inode);
    iput(inode);
    inode = NULL;

    if (unlikely(!target)) {
        log_err("cannot find target entity of '%s'\n", name);
        ret = UPROBE_RUN_OLD_FUNC;
        goto unlock_global_table;
    }

    /* step 2. find target latest patch */
    latest_patch = list_first_entry_or_null(&target->actived_list, struct patch_entity, actived_node);
    if (unlikely(!latest_patch)) {
        ret = UPROBE_RUN_OLD_FUNC;
        goto unlock_global_table;
    }

    mutex_lock(&target->process_mutex);

    /* step 3. collect all exited process */
    target_gather_exited_processes(target, &exited_proc_list);

    /* step 4. find or create process entity for current process */
    process = target_get_or_create_process(target);
    if (unlikely(!process)) {
        log_err("failed to get process of '%s'\n", target->meta.path);
        ret = UPROBE_RUN_OLD_FUNC;
        mutex_unlock(&target->process_mutex);
        goto unlock_global_table;
    }

    mutex_unlock(&target->process_mutex);

    /* step 5. check if we need resolve patch on the process */
    mutex_lock(&process->lock); // we want to ensure only one thread can resolve patch

    if (!process->latest_patch || process->latest_patch->patch != latest_patch) {
        loaded_patch = process_find_loaded_patch(process, latest_patch);
        if (loaded_patch) {
            log_debug("switch patch to '%s'\n", latest_patch->meta.path);
            process->latest_patch = loaded_patch;
        } else {
            log_debug("applying patch '%s' to process '%s' (pid=%d)\n", latest_patch->meta.path, name, pid);
            ret = upatch_resolve(target, latest_patch, process, vma_start);
            if (ret) {
                log_err("failed to apply patch '%s' to process '%s', ret=%d\n", latest_patch->meta.path, name, ret);
                ret = UPROBE_RUN_OLD_FUNC;
                goto unlock_process;
            }
        }
    }

    /* search and set pc register to new address */
    ret = jump_to_new_pc(regs, process->latest_patch, pc);

unlock_process:
    mutex_unlock(&process->lock);

unlock_global_table:
    up_read(&g_global_table_rwsem);

    free_exited_process(&exited_proc_list);
    return ret;
}

/* PATCH MANAGEMENT */
static void unregister_single_patch_function(struct target_entity *target, struct upatch_function *func)
{
    bool need_unregister = false;

    target_remove_function(target, func, &need_unregister);
    if (need_unregister) {
        uprobe_unregister(target->meta.inode, func->old_addr, &g_upatch_consumer);
    }
}

static void unregister_patch_functions(struct target_entity *target, struct patch_entity *patch, size_t count)
{
    struct upatch_function *funcs = patch->meta.funcs;
    const char *strings = patch->meta.strings;

    struct upatch_function *func;
    const char *name;
    size_t i;

    if (count > patch->meta.func_num) {
        log_err("function count %zu exceeds %zu\n", count, patch->meta.func_num);
        return;
    }

    log_debug("unregister patch '%s' functions:\n", target->meta.path);
    for (i = 0; i < count; i++) {
        func = &funcs[i];
        name = strings + func->name_off;

        log_debug("- function: offset=0x%08llx, size=0x%04llx, name='%s'\n", func->old_addr, func->old_size, name);
        unregister_single_patch_function(target, func);
    }
}

static int register_single_patch_function(struct target_entity *target, struct upatch_function *func)
{
    bool need_register = false;
    int ret;

    ret = target_add_function(target, func, &need_register);
    if (ret) {
        log_err("failed to add patch function to target\n");
        return ret;
    }

    if (need_register) {
        ret = uprobe_register(target->meta.inode, func->old_addr, &g_upatch_consumer);
        if (ret) {
            target_remove_function(target, func, &need_register); // rollback, remove function from target
            log_err("failed to register uprobe on '%s' (inode: %lu) at 0x%llx, ret=%d\n",
                target->meta.path, target->meta.inode->i_ino, func->old_addr, ret);
            return ret;
        }
    }

    return 0;
}

static int register_patch_functions(struct target_entity *target, struct patch_entity *patch, size_t count)
{
    struct upatch_function *funcs = patch->meta.funcs;
    const char *strings = patch->meta.strings;

    struct upatch_function *func;
    const char *name;
    size_t i;

    int ret;

    if (count > patch->meta.func_num) {
        log_err("function count %zu exceeds %zu\n", count, patch->meta.func_num);
        return -EINVAL;
    }

    log_debug("register target '%s' functions:\n", target->meta.path);
    for (i = 0; i < count; i++) {
        func = &funcs[i];
        name = strings + func->name_off;

        log_debug("+ function: offset=0x%08llx, size=0x%04llx, name='%s'\n", func->old_addr, func->old_size, name);
        ret = register_single_patch_function(target, func);
        if (ret) {
            log_err("failed to register function '%s'\n", name);
            unregister_patch_functions(target, patch, i);
            return ret;
        }
    }

    return 0;
}

/* public interface */
enum upatch_status upatch_status(const char *patch_file)
{
    enum upatch_status status = UPATCH_STATUS_NOT_APPLIED;
    struct patch_entity *patch = NULL;

    down_read(&g_global_table_rwsem);
    patch = find_patch(patch_file);
    if (patch) {
        status = patch->status;
    }
    up_read(&g_global_table_rwsem);

    return status;
}

int upatch_load(const char *patch_file, const char *target_file)
{
    struct patch_entity *patch = NULL;
    struct patch_entity *preload_patch = NULL;
    struct patch_entity *patch_to_free = NULL;
    struct target_entity *target = NULL;
    struct target_entity *preload_target = NULL;
    struct target_entity *target_to_free = NULL;
    int ret = 0;

    if (!patch_file || !target_file) {
        return -EINVAL;
    }

    log_debug("loading patch '%s' -> '%s'...\n", patch_file, target_file);

    /* fast path, return if patch already exists */
    down_read(&g_global_table_rwsem);
    if (find_patch(patch_file)) {
        log_err("patch '%s' is already exist\n", patch_file);
        ret = -EEXIST;
        up_read(&g_global_table_rwsem);
        goto out;
    }
    up_read(&g_global_table_rwsem);

    /* preload patch & target file out of the lock */
    preload_patch = new_patch_entity(patch_file);
    if (IS_ERR(preload_patch)) {
        log_err("failed to load patch '%s'\n", patch_file);
        ret = PTR_ERR(preload_patch);
        goto out_free;
    }

    preload_target = new_target_entity(target_file);
    if (IS_ERR(preload_target)) {
        log_err("failed to load target '%s'\n", target_file);
        patch_to_free = preload_patch;
        ret = PTR_ERR(preload_target);
        goto out_free;
    }

    /* slow path, load patch & target from file */
    down_write(&g_global_table_rwsem);

    /* step 1. recheck patch and target reference */
    patch = find_patch(patch_file);
    if (!patch) {
        patch = preload_patch;             // patch does not exist, use preloaded patch
    } else {
        log_err("patch '%s' is already exist\n", patch_file);
        ret = -EEXIST;
        patch_to_free = preload_patch;
        target_to_free = preload_target;
        goto unlock_global_table;
    }

    target = find_target(target_file);
    if (!target) {
        target = preload_target;           // target does not exist, use preloaded target
    } else {
        target_to_free = preload_target;   // target exists, need free preload one
    }

    if (target != preload_target) {
        down_write(&target->action_rwsem); // lock global target, patch is always local
    }

    /* step 2. add patch to global patch table */
    hash_add(g_patch_table, &patch->table_node, patch->meta.inode->i_ino);

    /* step 3. add patch to target all patches list */
    list_add(&patch->loaded_node, &target->loaded_list);

    /* step 4. update patch status */
    patch->target = target;
    patch->status = UPATCH_STATUS_DEACTIVED;

    if (target != preload_target) {
        up_write(&target->action_rwsem);   // unlock global target, patch is always local
    } else {
        /* step 5. add new target to global target table */
        hash_add(g_target_table, &target->table_node, target->meta.inode->i_ino);
    }

unlock_global_table:
    up_write(&g_global_table_rwsem);

out_free:
    if (patch_to_free) {
        free_patch_entity(patch_to_free);
    }
    if (target_to_free) {
        free_target_entity(target_to_free);
    }

out:
    if (!ret) {
        log_debug("patch '%s' is loaded\n", patch_file);
    }
    return ret;
}

int upatch_remove(const char *patch_file)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;
    struct patch_entity *patch_to_free = NULL;
    struct target_entity *target_to_free = NULL;

    int ret = 0;

    log_debug("removing patch '%s'...\n", patch_file);

    down_write(&g_global_table_rwsem);

    patch = find_patch(patch_file);
    if (!patch) {
        log_err("cannot find patch entity\n");
        ret = -ENOENT;
        goto unlock_global_table;
    }

    if (patch->status != UPATCH_STATUS_DEACTIVED) {
        log_err("invalid patch status\n");
        ret = -EPERM;
        goto unlock_global_table;
    }

    target = patch->target;
    if (!target) {
        log_err("cannot find target entity\n");
        ret = -EFAULT;
        goto unlock_global_table;
    }

    down_write(&target->action_rwsem);
    down_write(&patch->action_rwsem);

    /* step 1. check if the patch removable */
    ret = target_check_patch_removable(target, patch);
    if (ret) {
        log_err("patch %s is not removable\n", patch_file);
        goto unlock_target;
    }

    /* step 2. remove patch from from global table */
    hash_del(&patch->table_node);

    /* step 3. remove patch from target patch list */
    list_del_init(&patch->loaded_node);

    /* step 4. check & remove target form global table */
    if (list_empty(&target->loaded_list)) {
        hash_del(&target->table_node);
        target_to_free = target;
    }

    /* step 5. update patch status */
    patch->target = NULL;
    patch->status = UPATCH_STATUS_NOT_APPLIED;
    patch_to_free = patch;

unlock_target:
    up_write(&target->action_rwsem);

unlock_global_table:
    up_write(&g_global_table_rwsem);

    if (patch_to_free) {
        free_patch_entity(patch_to_free);
    }
    if (target_to_free) {
        free_target_entity(target_to_free);
    }
    if (!ret) {
        log_debug("patch '%s' is removed\n", patch_file);
    }
    return ret;
}

int upatch_active(const char *patch_file)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;
    int ret = 0;

    log_debug("activating patch '%s'...\n", patch_file);

    down_read(&g_global_table_rwsem);

    patch = find_patch(patch_file);
    if (!patch) {
        log_err("cannot find patch entity\n");
        ret = -ENOENT;
        goto unlock_global_table;
    }

    if (patch->status != UPATCH_STATUS_DEACTIVED) {
        log_err("invalid patch status\n");
        ret = -EPERM;
        goto unlock_global_table;
    }

    target = patch->target;
    if (!target) {
        log_err("cannot find target entity\n");
        ret = -EFAULT;
        goto unlock_global_table;
    }

    down_write(&target->action_rwsem);
    down_write(&patch->action_rwsem);

    /* step 1. register patch functions to target */
    ret = register_patch_functions(target, patch, patch->meta.func_num);
    if (ret) {
        log_err("failed to register patch functions\n");
        goto unlock_entity;
    }

    /* step 2. add patch to target actived patch list */
    list_add(&patch->actived_node, &target->actived_list);

    /* step 3. update patch status */
    patch->status = UPATCH_STATUS_ACTIVED;

unlock_entity:
    up_write(&patch->action_rwsem);
    up_write(&target->action_rwsem);

unlock_global_table:
    up_read(&g_global_table_rwsem);

    if (!ret) {
        log_debug("patch '%s' is actived\n", patch_file);
    }
    return ret;
}

int upatch_deactive(const char *patch_file)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;
    int ret = 0;

    log_debug("deactivating patch '%s'...\n", patch_file);

    down_read(&g_global_table_rwsem);

    patch = find_patch(patch_file);
    if (!patch) {
        log_err("cannot find patch entity\n");
        ret = -ENOENT;
        goto unlock_global_table;
    }

    if (patch->status != UPATCH_STATUS_ACTIVED) {
        log_err("invalid patch status\n");
        ret = -EPERM;
        goto unlock_global_table;
    }

    target = patch->target;
    if (!target) {
        log_err("cannot find target entity\n");
        ret = -EFAULT;
        goto unlock_global_table;
    }

    down_write(&target->action_rwsem);
    down_write(&patch->action_rwsem);

    /* step 1. remove patch functions from target */
    unregister_patch_functions(target, patch, patch->meta.func_num);

    /* step 2. remove patch from target actived patch list */
    list_del_init(&patch->actived_node);

    /* step 3. update patch status */
    patch->status = UPATCH_STATUS_DEACTIVED;

    up_write(&patch->action_rwsem);
    up_write(&target->action_rwsem);

unlock_global_table:
    up_read(&g_global_table_rwsem);

    if (!ret) {
        log_debug("patch '%s' is deactived\n", patch_file);
    }
    return ret;
}

void __exit report_global_table_populated(void)
{
    struct patch_entity *patch;
    struct target_entity *target;
    int bkt;

    down_read(&g_global_table_rwsem);
    hash_for_each(g_patch_table, bkt, patch, table_node) {
        log_warn("found patch '%s' on exit, status=%s",
            patch->meta.path ? patch->meta.path : "(null)", patch_status_str(patch->status));
    }
    hash_for_each(g_target_table, bkt, target, table_node) {
        log_err("found target '%s' on exit", target->meta.path ? target->meta.path : "(null)");
    }
    up_read(&g_global_table_rwsem);
}
