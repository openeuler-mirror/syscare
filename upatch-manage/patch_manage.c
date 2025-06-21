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

#include "patch_entity.h"
#include "target_entity.h"
#include "process_entity.h"
#include "patch_load.h"
#include "util.h"

#ifndef UPROBE_ALTER_PC
#define UPROBE_ALTER_PC 2
#endif

#define UPROBE_RUN_OLD_FUNC 0

static int jump_to_new_pc(struct pt_regs *regs, struct patch_info *info, unsigned long old_pc)
{
    struct pc_pair *pp;

    bool find = false;
    hash_for_each_possible(info->pc_maps, pp, node, old_pc) {
        if (pp->old_pc == old_pc) {
            find = true;
            break;
        }
    }

    if (!find) {
        log_err("cannot find new pc for 0x%lx\n", old_pc);
        return UPROBE_RUN_OLD_FUNC;
    }

    log_debug("jump from 0x%lx -> 0x%lx\n", old_pc, pp->new_pc);

    instruction_pointer_set(regs, pp->new_pc);
    return UPROBE_ALTER_PC;
}

static struct file *get_target_file_from_pc(struct mm_struct *mm, unsigned long pc, unsigned long *code_start)
{
    struct vm_area_struct *vma = NULL;

    vma = find_vma(mm, pc);
    if (!vma) {
        return NULL;
    }

    if (!vma->vm_file) {
        return NULL;
    }

    *code_start = vma->vm_start;

    return vma->vm_file;
}

static struct target_entity *get_target_from_pc(unsigned long pc, unsigned long *code_start)
{
    struct target_entity *target;

    struct mm_struct *mm = current->mm;
    struct file *target_file;

    mmap_read_lock(mm);
    target_file = get_target_file_from_pc(mm, pc, code_start);
    mmap_read_unlock(mm);

    if (!target_file) {
        log_err("no backen file found for upatch\n");
        return NULL;
    }

    get_file(target_file);
    target = get_target_entity_by_inode(file_inode(target_file));
    fput(target_file);

    if (!target) {
        log_err("no target found in patch handler\n");
        return NULL;
    }

    return target;
}

static struct patch_info* find_loaded_patches(struct process_entity *pro, struct patch_entity *patch)
{
    struct patch_info* info;
    list_for_each_entry(info, &pro->loaded_patches, list) {
        if (info->patch == patch) {
            return info;
        }
    }
    return NULL;
}

// UPROBE_RUN_OLD_FUNC means run old func
// UPROBE_ALTER_PC means run new func
static int upatch_uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
    unsigned long pc = instruction_pointer(regs);
    struct target_entity *target;
    struct patch_entity *actived_patch;
    struct process_entity *process;
    int ret = UPROBE_RUN_OLD_FUNC;
    const char *name = current->comm;
    pid_t pid = task_pid_nr(current);
    pid_t tgid = task_tgid_nr(current);
    unsigned long target_code_start;

    log_debug("upatch handler triggered on process '%s' (pid=%d, tgid=%d, pc=0x%lx)\n", name, pid, tgid, pc);

    target = get_target_from_pc(pc, &target_code_start);
    if (!target) {
        log_err("cannot find target entity of '%s'\n", name);
        return ret;
    }

    down_read(&target->patch_lock);
    actived_patch = list_first_entry(&target->actived_patch_list, struct patch_entity, actived_node);
    up_read(&target->patch_lock);

    if (!actived_patch) {
        log_err("cannot find any actived patch of '%s'\n", name);
        return ret;
    }

    process = get_process(target);
    if (!process) {
        return UPROBE_RUN_OLD_FUNC;
    }

    // multi thread may trap at the same time, only one thread can load patch, other thread should wait
    mutex_lock(&process->lock);

    if (!process->active_info) {
        log_debug("applying new patch '%s' to '%s' (pid=%d, tgid=%d)...\n",
            actived_patch->path, name, pid, tgid);
        ret = upatch_resolve(target, actived_patch, process, target_code_start);
        if (ret) {
            log_err("failed to apply patch '%s' to '%s', ret=%d\n", actived_patch->path, name, ret);
            goto fail;
        }
    } else if (process->active_info->patch != actived_patch) {
        struct patch_info* info = find_loaded_patches(process, actived_patch);
        if (info) {
            process->active_info = info;
            goto ok;
        }

        log_debug("applying latest patch '%s' to '%s' (pid=%d, tgid=%d)...\n",
            actived_patch->path, name, pid, tgid);
        ret = upatch_resolve(target, actived_patch, process, target_code_start);
        if (ret) {
            log_err("failed to apply patch '%s' to '%s', ret=%d\n", actived_patch->path, name, ret);
            goto fail;
        }
    }

ok:
    ret = jump_to_new_pc(regs, process->active_info, pc);
    mutex_unlock(&process->lock);
    return ret;

fail:
    mutex_unlock(&process->lock);
    return UPROBE_RUN_OLD_FUNC;
}

static struct uprobe_consumer patch_consumer = {
    .handler = upatch_uprobe_handler,
};

// register uprobe if offset of this target have no new function
static int target_register_function(struct target_entity *target, loff_t offset,
    struct upatch_function *func)
{
    struct patched_offset *off = NULL;
    struct patched_func_node *func_node;
    bool find = false;
    int ret;

    func_node = kzalloc(sizeof(struct patched_func_node), GFP_KERNEL);
    if (!func_node) {
        return -ENOMEM;
    }

    // find if this target have func changed in offset
    list_for_each_entry(off, &target->offset_node, list) {
        if (off->offset == offset) {
            find = true;
            break;
        }
    }

    // This is the first func in this offset, so we should create a off node
    if (!find) {
        off = kzalloc(sizeof(struct patched_offset), GFP_KERNEL);
        if (!off) {
            kfree(func_node);
            return -ENOMEM;
        }

        off->offset = offset;
        INIT_LIST_HEAD(&off->funcs_head);

        log_debug("register uprobe on '%s' (inode: %lu) at 0x%llx\n",
            target->path, target->inode->i_ino, offset);
        ret = uprobe_register(target->inode, offset, &patch_consumer);
        if (ret) {
            log_err("failed to register uprobe on '%s' (inode: %lu) at 0x%llx, ret=%d\n",
                target->path, target->inode->i_ino, offset, ret);
            kfree(func_node);
            kfree(off);
            return ret;
        }

        list_add(&off->list, &target->offset_node);
    }

    func_node->func = func;
    list_add(&func_node->list, &off->funcs_head);
    return 0;
}

// unregister uprobe if offset of this target have no new function
static void unregister_function_uprobe(struct target_entity *target, loff_t offset, struct upatch_function *func)
{
    struct patched_offset *off = NULL;
    struct patched_func_node *func_node = NULL;
    struct patched_func_node *tmp = NULL;
    bool find = false;

    list_for_each_entry(off, &target->offset_node, list) {
        if (off->offset == offset) {
            find = true;
            break;
        }
    }

    if (!find) {
        log_err("cannot find offest 0x%llx of '%s'\n", offset, target->path);
        return;
    }

    // There may be multiple version func in the same offset of a target. We should find it and delete it
    list_for_each_entry_safe(func_node, tmp, &off->funcs_head, list) {
        if (func_node->func == func) {
            list_del(&func_node->list);
            kfree(func_node);
        }
    }

    if (list_empty(&off->funcs_head)) {
        uprobe_unregister(target->inode, offset, &patch_consumer);
        list_del(&off->list);
        kfree(off);
    }
}

static void target_unregister_functions(struct target_entity *target, struct patch_entity *patch,
    struct upatch_function *funcs, size_t count)
{
    struct upatch_function *func = NULL;
    size_t i = 0;
    loff_t offset = 0;
    const char *name = NULL;

    log_debug("unregister patch '%s' functions:\n", target->path);
    for (i = 0; i < count; i++) {
        func = &funcs[i];
        offset = func->old_addr;
        name = patch->meta.strings + func->name_off;

        log_debug("- function: offset=0x%08llx, size=0x%04llx, name='%s'\n", offset, func->old_size, name);
        unregister_function_uprobe(target, offset, func);
    }
}

// patch will be actived in uprobe handler
static int do_active_patch(struct target_entity *target, struct patch_entity *patch)
{
    struct upatch_function *funcs = patch->meta.funcs;
    struct upatch_function *func;
    int ret = 0;
    size_t i = 0;
    loff_t offset;
    const char *name = NULL;

    log_debug("register target '%s' functions:\n", target->path);
    down_write(&target->patch_lock);

    for (i = 0; i < patch->meta.func_num; i++) {
        func = &funcs[i];
        offset = func->old_addr;
        name = patch->meta.strings + func->name_off;

        log_debug("+ function: offset=0x%08llx, size=0x%04llx, name='%s'\n", offset, func->old_size, name);
        ret = target_register_function(target, offset, func);
        if (ret) {
            log_err("failed to register function '%s', ret=%d\n", name, ret);
            target_unregister_functions(target, patch, funcs, i);
            goto out;
        }
    }

    list_add(&patch->actived_node, &target->actived_patch_list);
    patch->status = UPATCH_STATUS_ACTIVED;

out:
    up_write(&target->patch_lock);

    return ret;
}

static void target_remove_actived_patch(struct target_entity *target, struct patch_entity *patch)
{
    struct patch_entity *p = NULL;
    struct patch_entity *tmp = NULL;
    bool found = false;

    list_for_each_entry_safe(p, tmp, &target->actived_patch_list, actived_node) {
        if (p == patch) {
            list_del_init(&p->actived_node);
            found = true;
            break;
        }
    }

    if (!found) {
        log_err("cannot find actived patch '%s'\n", patch->path);
    }
}

// delete patch inode & function in target
// patch will be deactived in uprobe handler
static void do_deactive_patch(struct patch_entity *patch)
{
    struct target_entity *target = patch->target;
    struct upatch_function *funcs = patch->meta.funcs;

    down_write(&target->patch_lock);

    target_unregister_functions(target, patch, funcs, patch->meta.func_num);
    target_remove_actived_patch(target, patch);
    patch->status = UPATCH_STATUS_DEACTIVED;

    up_write(&target->patch_lock);
}

/* public interface */
enum upatch_status upatch_status(const char *patch_file)
{
    struct patch_entity *patch;

    patch = get_patch_entity(patch_file);
    return patch ? patch->status : UPATCH_STATUS_NOT_APPLIED;
}

int upatch_load(const char *patch_file, const char *target_path)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;

    if (!patch_file || !target_path) {
        return -EINVAL;
    }

    log_debug("loading patch '%s' -> '%s'...\n", patch_file, target_path);

    patch = get_patch_entity(patch_file);
    if (patch) {
        log_err("patch '%s' is already loaded\n", patch_file);
        return -EEXIST;
    }

    patch = new_patch_entity(patch_file);
    if (IS_ERR(patch)) {
        log_err("failed to load patch '%s'\n", patch_file);
        return PTR_ERR(patch);
    }

    target = get_target_entity(target_path);
    if (!target) {
        target = new_target_entity(target_path);
        if (IS_ERR(target)) {
            free_patch_entity(patch);
            log_err("failed to load target '%s'\n", target_path);
            return PTR_ERR(target);
        }
    }

    list_add(&patch->patch_node, &target->all_patch_list);
    patch->target = target;
    patch->status = UPATCH_STATUS_DEACTIVED;

    log_debug("patch '%s' is loaded\n", patch_file);
    return 0;
}

int upatch_remove(const char *patch_file)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;

    log_debug("removing patch '%s'...\n", patch_file);

    patch = get_patch_entity(patch_file);
    if (!patch) {
        log_err("cannot find patch entity '%s'\n", patch_file);
        return -ENOENT;
    }

    if (patch->status != UPATCH_STATUS_DEACTIVED) {
        log_err("invalid patch status\n");
        return -EPERM;
    }

    target = patch->target;

    free_patch_entity(patch);
    if (!is_target_has_patch(target)) {
        free_target_entity(target);
    }

    log_debug("patch '%s' is removed\n", patch_file);
    return 0;
}

int upatch_active(const char *patch_file)
{
    struct patch_entity *patch = NULL;
    struct target_entity *target = NULL;
    int ret;

    log_debug("activating patch '%s'...\n", patch_file);

    patch = get_patch_entity(patch_file);
    if (!patch) {
        log_err("cannot find patch entity '%s'\n", patch_file);
        return -ENOENT;
    }

    // check patch status
    if (patch->status != UPATCH_STATUS_DEACTIVED) {
        log_err("invalid patch status\n");
        return -EPERM;
    }

    target = patch->target;

    ret = do_active_patch(target, patch);
    if (ret) {
        log_err("failed to active patch '%s', ret=%d\n", patch_file, ret);
        return ret;
    }

    log_debug("patch '%s' is actived\n", patch_file);
    return 0;
}

int upatch_deactive(const char *patch_file)
{
    struct patch_entity *patch = NULL;

    log_debug("deactivating patch '%s'...\n", patch_file);

    // find patch
    patch = get_patch_entity(patch_file);
    if (!patch) {
        log_err("cannot find patch entity '%s'\n", patch_file);
        return -ENOENT;
    }

    // check patch status
    if (patch->status != UPATCH_STATUS_ACTIVED) {
        log_err("invalid patch status\n");
        return -EPERM;
    }

    do_deactive_patch(patch);

    log_debug("patch '%s' is deactived\n", patch_file);
    return 0;
}

void target_unregister_uprobes(struct target_entity *target)
{
    struct patched_offset *off = NULL;
    struct patched_offset *tmp_off = NULL;

    log_debug("unregister '%s' (inode: %lu) uprobes:", target->path, target->inode->i_ino);
    list_for_each_entry_safe(off, tmp_off, &target->offset_node, list) {
        log_debug("unregister offset 0x%llx\n", off->offset);
        uprobe_unregister(target->inode, off->offset, &patch_consumer);
        list_del(&off->list);
        kfree(off);
    }
}
