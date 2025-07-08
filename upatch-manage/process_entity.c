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

#include <linux/mm.h>
#include <linux/sched/task.h>

#include "patch_entity.h"
#include "target_entity.h"

#include "patch_load.h"
#include "util.h"

static int do_free_patch_memory(struct mm_struct *mm, unsigned long addr, size_t len)
{
    struct vm_area_struct *vma;

    if (!addr) {
        return -EINVAL;
    }

    if (!len) {
        return 0;
    }

    vma = find_vma(mm, addr);
    if (unlikely(!vma)) {
        return -ENOENT;
    }

    if (unlikely(vma->vm_start != addr || (vma->vm_end - vma->vm_start) != len)) {
        return -EFAULT;
    }

    return do_munmap(mm, addr, len, NULL);
}

static void free_patch_memory(struct task_struct *task, struct patch_info *patch)
{
    pid_t pid = task_pid_nr(task);
    struct mm_struct *mm;

    int ret;

    mm = get_task_mm(task);
    if (unlikely(!mm)) {
        return;
    }

    mmap_write_lock(mm);

    log_debug("process %d: free patch text, addr=0x%lx, len=0x%lx\n",
        pid, patch->text_addr, patch->text_len);
    ret = do_free_patch_memory(mm, patch->text_addr, patch->text_len);
    if (ret) {
        log_err("failed to free patch text, pid=%d, addr=0x%lx, len=0x%lx, ret=%d\n",
            pid, patch->text_addr, patch->text_len, ret);
    }

    log_debug("process %d: free patch rodata, addr=0x%lx, len=0x%lx\n",
        pid, patch->rodata_addr, patch->rodata_len);
    ret = do_free_patch_memory(mm, patch->rodata_addr, patch->rodata_len);
    if (ret) {
        log_err("failed to free patch rodata, pid=%d, addr=0x%lx, len=0x%lx, ret=%d\n",
            pid, patch->rodata_addr, patch->rodata_len, ret);
    }

    mmap_write_unlock(mm);

    mmput(mm);
    return;
}

static void free_patch_info(struct patch_info *patch)
{
    struct pc_pair *pair;
    struct hlist_node *tmp;
    int bkt;

    if (unlikely(!patch)) {
        return;
    }

    hash_for_each_safe(patch->pc_maps, bkt, tmp, pair, node) {
        hash_del(&pair->node);
        kfree(pair);
    }

    kfree(patch);
}

struct process_entity *new_process(struct target_entity *target)
{
    if (unlikely(!target)) {
        return ERR_PTR(-EINVAL);
    }

    struct process_entity *process = kzalloc(sizeof(struct process_entity), GFP_KERNEL);
    if (!process) {
        return ERR_PTR(-ENOMEM);
    }

    process->pid = get_task_pid(current, PIDTYPE_TGID);
    if (!process->pid) {
        log_err("failed to get process %d task pid\n", task_tgid_nr(current));
        kfree(process);
        return ERR_PTR(-EFAULT);
    }
    process->task = get_task_struct(current);

    mutex_init(&process->lock);

    process->latest_patch = NULL;
    INIT_LIST_HEAD(&process->loaded_patches);
    INIT_LIST_HEAD(&process->process_node);

    return process;
}

void free_process(struct process_entity *process)
{
    pid_t pid;
    struct patch_info *patch;
    struct patch_info *tmp;

    if (unlikely(!process)) {
        return;
    }

    pid = task_pid_nr(process->task);

    log_debug("free process %d\n", pid);
    list_for_each_entry_safe(patch, tmp, &process->loaded_patches, node) {
        list_del(&patch->node);
        free_patch_memory(process->task, patch);
        free_patch_info(patch);
    }

    put_pid(process->pid);
    put_task_struct(process->task);

    kfree(process);
}

struct patch_info *process_find_loaded_patch(struct process_entity *process, struct patch_entity *patch)
{
    struct patch_info *curr_patch;

    list_for_each_entry(curr_patch, &process->loaded_patches, node) {
        if (curr_patch->patch == patch) {
            return curr_patch;
        }
    }

    return NULL;
}

int process_write_patch_info(struct process_entity *process, struct patch_entity *patch, struct patch_context *ctx)
{
    struct upatch_function *funcs = (struct upatch_function *)ctx->func_shdr->sh_addr;
    size_t func_num = ctx->func_shdr->sh_size / (sizeof (struct upatch_function));

    struct upatch_relocation *relas = (struct upatch_relocation *)ctx->rela_shdr->sh_addr;
    const char *strings = (const char *)ctx->string_shdr->sh_addr;

    size_t i;
    struct upatch_function *func;
    const char *func_name;

    struct patch_info *info;
    struct pc_pair *entry;

    info = kzalloc(sizeof(struct patch_info), GFP_KERNEL);
    if (!info) {
        log_err("failed to alloc patch info\n");
        return -ENOMEM;
    }

    hash_init(info->pc_maps);
    for (i = 0; i < func_num; ++i) {
        func = &funcs[i];

        entry = kmalloc(sizeof(*entry), GFP_KERNEL);
        if (!entry) {
            free_patch_info(info);
            return -ENOMEM;
        }

        func_name = strings + relas[i].name.r_addend;
        entry->old_pc = funcs[i].old_addr + ctx->load_bias + ctx->target->load_offset;
        entry->new_pc = funcs[i].new_addr;
        hash_add(info->pc_maps, &entry->node, entry->old_pc);
        log_debug("function: 0x%08lx -> 0x%08lx, name: '%s'\n", entry->old_pc, entry->new_pc, func_name);
    }

    info->patch = patch;

    info->text_addr = ctx->layout.base;
    info->text_len = ctx->layout.text_end;

    info->rodata_addr = ctx->layout.base + ctx->layout.text_end;
    info->rodata_len = ctx->layout.ro_after_init_end - ctx->layout.text_end;

    list_add(&info->node, &process->loaded_patches);
    process->latest_patch = info;

    return 0;
}
