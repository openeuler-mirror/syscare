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
#include <linux/hash.h>
#include <linux/hashtable.h>
#include <linux/spinlock.h>

#include "target_entity.h"
#include "process_entity.h"
#include "patch_entity.h"
#include "patch_load.h"
#include "util.h"

#define UPROBE_RUN_OLD_FUNC 0

#ifndef UPROBE_ALTER_PC
#define UPROBE_ALTER_PC 2
#endif

#define TARGET_FILE_HASH_BITS 5 // would have less than 2^5 = 32 targets

/* --- Forward declarations --- */

static int upatch_uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs);

/* --- Global variables --- */

static DEFINE_HASHTABLE(g_target_table, TARGET_FILE_HASH_BITS); // global target hash table
static DEFINE_SPINLOCK(g_target_table_lock); // lock for global target hash table, SHOULD NOT hold other lock inside it

static struct uprobe_consumer g_uprobe_consumer = {
    .handler = upatch_uprobe_handler,
};

/* --- Target table management --- */

static inline struct target_entity *find_target_unlocked(struct inode *inode)
{
    struct target_entity *target;

    hash_for_each_possible(g_target_table, target, node, hash_inode(inode, TARGET_FILE_HASH_BITS)) {
        if (inode_equal(target->file.inode, inode)) {
            return target;
        }
    }

    return NULL;
}

static inline struct target_entity *get_target_by_inode(struct inode *inode)
{
    struct target_entity *target;

    spin_lock(&g_target_table_lock);
    target = find_target_unlocked(inode);
    get_target(target);
    spin_unlock(&g_target_table_lock);

    return target;
}

/* --- Uprobe management --- */

static struct inode *get_vma_file_inode(unsigned long pc, unsigned long *vma_start)
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

static int upatch_uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    const char *proc_name = current->comm;
    pid_t tgid = task_tgid_nr(current);
    pid_t pid = task_pid_nr(current);

    unsigned long pc = instruction_pointer(regs);
    unsigned long vma_start = 0;

    struct inode *inode = NULL;
    struct target_entity *target = NULL;
    struct process_entity *process = NULL;
    struct patch_entity *actived_patch = NULL;

    struct patch_info *patch_info;
    unsigned long jump_addr;

    int ret = UPROBE_RUN_OLD_FUNC;

    log_debug("uprobe handler triggered on process '%s' (tgid=%d, pid=%d, pc=0x%lx)\n", proc_name, tgid, pid, pc);

    /* Step 1: Get vma corresponding file inode */
    inode = get_vma_file_inode(pc, &vma_start);
    if (unlikely(!inode)) {
        log_err("cannot get vma file inode of '%s'\n", proc_name);
        goto release_out;
    }

    /* Step 2: Get target entity by vma file inode */
    target = get_target_by_inode(inode);
    if (unlikely(!target)) {
        log_err("cannot find target entity of '%s'\n", proc_name);
        goto release_out;
    }

    /* Step 3: Clean up target exited proesses */
    target_cleanup_process(target);

    /* Step 4: Get process entity for current process */
    process = target_get_process(target, current);
    if (unlikely(IS_ERR(process))) {
        log_err("failed to get '%s' process, ret=%d\n", target->file.path, (int)PTR_ERR(process));
        goto release_out;
    }

    spin_lock(&process->thread_lock); // ensure only one thread could check & resolve patch

    /* Step 5: Get actived patch entity of the target */
    actived_patch = target_get_actived_patch(target);
    if (unlikely(!actived_patch)) {
        // target does not have any actived patch yet
        goto unlock_out;
    }

    /* Step 6. Check or resolve patch of the process */
    patch_info = process_switch_and_get_patch(process, actived_patch);
    if (unlikely(!patch_info)) {
        ret = upatch_resolve(target, actived_patch, process, vma_start);
        if (unlikely(ret)) {
            log_err("process %d: failed to resolve patch %s, ret=%d\n", tgid, actived_patch->file.path, ret);
            goto unlock_out;
        }
    }

    /* Step 7: Find patch function jump addr */
    jump_addr = process_get_jump_addr(process, pc);
    if (unlikely(!jump_addr)) {
        log_err("process %d: cannot find jump address, pc=0x%lx\n", tgid, pc);
        goto unlock_out;
    }

    /* Step 8: Set patch function jump addr to pc register */
    instruction_pointer_set(regs, jump_addr);
    log_debug("process %d: jump 0x%lx -> 0x%lx\n", tgid, pc, jump_addr);

unlock_out:
    spin_unlock(&process->thread_lock);

release_out:
    put_patch(actived_patch);
    put_process(process);
    put_target(target);
    iput(inode);

    return unlikely(ret) ? UPROBE_RUN_OLD_FUNC : UPROBE_ALTER_PC;
}

/* --- Public interface --- */

int upatch_load(const char *target_file, const char *patch_file)
{
    struct target_entity *target;
    struct file *file;

    struct target_entity *new_target = NULL;
    struct target_entity *found_target = NULL;
    int ret = 0;

    if (unlikely(!target_file || !patch_file)) {
        return -EINVAL;
    }

    log_debug("%s: loading patch %s...\n", target_file, patch_file);

    /* Step 1: Open target file */
    file = filp_open(target_file, O_RDONLY, 0);
    if (unlikely(IS_ERR(file))) {
        return PTR_ERR(file);
    }

    /* Step 2: Check and get target entity */
    spin_lock(&g_target_table_lock);
    target = get_target(find_target_unlocked(file_inode(file)));
    spin_unlock(&g_target_table_lock);

    if (!target) {
        /* Step 3: Load target from file */
        new_target = load_target(file);
        if (unlikely(IS_ERR(new_target))) {
            ret = PTR_ERR(new_target);
            new_target = NULL;
            log_err("failed to load target %s, ret=%d\n", target_file, ret);
            goto release_out;
        }

        /* Step 4: Re-check if the target exists (to handle race) */
        spin_lock(&g_target_table_lock);

        found_target = find_target_unlocked(file_inode(file));
        if (likely(!found_target)) {
            // nobody inserted the target during load process, insert new target into hash table
            target = new_target;
            new_target = NULL;
            hash_add(g_target_table, &target->node, hash_inode(file_inode(file), TARGET_FILE_HASH_BITS));
        } else {
            // someone already inserted the target, use founded one and free the target we load
            target = found_target;
        }

        /* Step 5: Get target reference */
        get_target(target);

        spin_unlock(&g_target_table_lock);
    }

    /* Step 6: Load patch to the target */
    ret = target_load_patch(target, patch_file);
    if (unlikely(ret)) {
        log_err("failed to load patch %s, ret=%d\n", patch_file, ret);
        goto release_out;
    }

    /* Step 7: Increase current module reference to eunsure it won't be removed */
    try_module_get(THIS_MODULE);

    log_debug("%s: patch %s is loaded\n", target_file, patch_file);

release_out:
    /* Step 8: Close opened file */
    filp_close(file, NULL);

    /* Step 9: Release all references we hold */
    put_target(target);      // reference of target hash table
    put_target(new_target);  // reference of we load

    return ret;
}

int upatch_remove(const char *target_file, const char *patch_file)
{
    struct inode *target_inode;
    struct inode *patch_inode;

    struct target_entity *target = NULL;
    struct target_entity *target_to_free = NULL;
    int ret = 0;

    if (unlikely(!target_file || !patch_file)) {
        return -EINVAL;
    }

    log_debug("%s: removing patch %s...\n", target_file, patch_file);

    /* Step 1: Get target & patch file inode */
    target_inode = get_path_inode(target_file);
    patch_inode = get_path_inode(patch_file);
    if (unlikely(!target_inode || !patch_inode)) {
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 2: Get target entity */
    spin_lock(&g_target_table_lock);
    target = get_target(find_target_unlocked(target_inode));
    spin_unlock(&g_target_table_lock);
    if (unlikely(!target)) {
        log_err("cannot find target entity\n");
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 3: Remove patch from the target */
    ret = target_remove_patch(target, patch_inode);
    if (unlikely(ret)) {
        log_err("%s: failed to remove patch %s, ret=%d\n", target_file, patch_file, ret);
        goto release_out;
    }

    log_debug("%s: patch %s is removed\n", target_file, patch_file);

    /* Step 4: Remove target when last patch was removed */
    mutex_lock(&target->patch_lock);
    spin_lock(&g_target_table_lock);
    if (hash_empty(target->patches) && !target->is_deleting) {
        hash_del(&target->node);
        target->is_deleting = true;
        target_to_free = target;
    }
    spin_unlock(&g_target_table_lock);
    mutex_unlock(&target->patch_lock);

    /* Step 5: Decrease current module reference */
    module_put(THIS_MODULE);

release_out:
    /* Step 6: Release all references we hold */
    put_target(target_to_free); // reference of target hash table
    put_target(target);         // reference of function context
    iput(patch_inode);
    iput(target_inode);

    return ret;
}

int upatch_active(const char *target_file, const char *patch_file)
{
    struct inode *target_inode;
    struct inode *patch_inode;

    struct target_entity *target = NULL;
    int ret = 0;

    if (unlikely(!target_file || !patch_file)) {
        return -EINVAL;
    }

    log_debug("%s: activating patch %s...\n", target_file, patch_file);

    /* Step 1: Get target & patch file inode */
    target_inode = get_path_inode(target_file);
    patch_inode = get_path_inode(patch_file);
    if (unlikely(!target_inode || !patch_inode)) {
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 2: Get target entity */
    spin_lock(&g_target_table_lock);
    target = get_target(find_target_unlocked(target_inode));
    spin_unlock(&g_target_table_lock);
    if (unlikely(!target)) {
        log_err("cannot find target entity\n");
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 3: Active patch on the target */
    ret = target_active_patch(target, patch_inode, &g_uprobe_consumer);
    if (unlikely(ret)) {
        log_err("%s: failed to active patch %s, ret=%d\n", target_file, patch_file, ret);
        goto release_out;
    }

    log_debug("%s: patch %s is actived\n", target_file, patch_file);

release_out:
    /* Step 4: Release all references we hold */
    put_target(target);
    iput(patch_inode);
    iput(target_inode);

    return ret;
}

int upatch_deactive(const char *target_file, const char *patch_file)
{
    struct inode *target_inode;
    struct inode *patch_inode;

    struct target_entity *target = NULL;
    int ret = 0;

    if (unlikely(!target_file || !patch_file)) {
        return -EINVAL;
    }

    log_debug("%s: deactivating patch %s...\n", target_file, patch_file);

    /* Step 1: Get target & patch file inode */
    target_inode = get_path_inode(target_file);
    patch_inode = get_path_inode(patch_file);
    if (unlikely(!target_inode || !patch_inode)) {
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 2: Get target entity */
    spin_lock(&g_target_table_lock);
    target = get_target(find_target_unlocked(target_inode));
    spin_unlock(&g_target_table_lock);
    if (unlikely(!target)) {
        log_err("cannot find target entity\n");
        ret = -ENOENT;
        goto release_out;
    }

    /* Step 3: Deactive patch on the target */
    ret = target_deactive_patch(target, patch_inode, &g_uprobe_consumer);
    if (unlikely(ret)) {
        log_err("%s: failed to deactive patch %s, ret=%d\n", target_file, patch_file, ret);
        goto release_out;
    }

    log_debug("%s: patch %s is deactived\n", target_file, patch_file);

release_out:
    /* Step 4: Release all references we hold */
    put_target(target);
    iput(patch_inode);
    iput(target_inode);

    return ret;
}

enum upatch_status upatch_status(const char *target_file, const char *patch_file)
{
    struct inode *target_inode;
    struct inode *patch_inode;

    struct target_entity *target = NULL;
    enum upatch_status status = UPATCH_STATUS_NOT_APPLIED;

    if (unlikely(!target_file || !patch_file)) {
        return status;
    }

    /* Step 1: Get target & patch file inode */
    target_inode = get_path_inode(target_file);
    patch_inode = get_path_inode(patch_file);
    if (unlikely(!target_inode || !patch_inode)) {
        goto release_out;
    }

    /* Step 2: Get target entity */
    spin_lock(&g_target_table_lock);
    target = get_target(find_target_unlocked(target_inode));
    spin_unlock(&g_target_table_lock);

    /* Step 3: Get patch status */
    status = target_patch_status(target, patch_inode);

release_out:
    /* Step 4: Release all references we hold */
    put_target(target);
    iput(patch_inode);
    iput(target_inode);

    return status;
}

void __exit check_target_table_populated(void)
{
    spin_lock(&g_target_table_lock);
    WARN_ON(!hash_empty(g_target_table));
    spin_unlock(&g_target_table_lock);
}
