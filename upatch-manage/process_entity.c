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

#include "kernel_compat.h"
#include "process_cache.h"

#include "patch_entity.h"
#include "target_entity.h"

#include "patch_load.h"
#include "stack_check.h"

#include "util.h"

/* --- Process life-cycle management --- */

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

    return upatch_munmap(mm, addr, len, NULL);
}

static void free_patch_memory(struct task_struct *task, struct patch_info *patch)
{
    pid_t pid = task_tgid_nr(task);
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

static void free_patch_info(struct patch_info *patch_info)
{
    struct patch_jump_entry *entry;
    struct hlist_node *tmp;
    int bkt;

    if (unlikely(!patch_info)) {
        return;
    }

    hash_for_each_safe(patch_info->jump_table, bkt, tmp, entry, node) {
        hash_del(&entry->node);
        kmem_cache_free(g_jump_entry_cache, entry);
    }

    put_patch(patch_info->patch);
    patch_info->patch = NULL;

    kmem_cache_free(g_patch_info_cache, patch_info);
}

static struct patch_info *find_patch_info_unlocked(struct process_entity *process, struct patch_entity *patch)
{
    struct patch_info *patch_info;
    struct patch_info *found = NULL;

    if (unlikely(!process || !patch)) {
        return NULL;
    }

    list_for_each_entry(patch_info, &process->patch_list, node) {
        if (patch_info->patch == patch) {
            found = patch_info;
            break;
        }
    }

    return found;
}

static bool is_patch_removable(pid_t pid, const void *page, void *context)
{
    static const size_t VALUE_NR = PAGE_SIZE / sizeof(unsigned long);

    const struct patch_info *patch = context;
    const unsigned long *stack_page = page;
    unsigned long stack_value;
    size_t i;

    struct patch_jump_entry *entry;
    int bkt;

    if (unlikely(patch->jump_min_addr >= patch->jump_max_addr)) {
        return true;
    }

    for (i = 0; i < VALUE_NR; i++) {
        stack_value = stack_page[i];

        /* Filter value does not in jump table range */
        if (likely(stack_value < patch->jump_min_addr || stack_value >= patch->jump_max_addr)) {
            continue;
        }

        /* Check if value is on the jump table */
        hash_for_each(patch->jump_table, bkt, entry, node) {
            if (unlikely(stack_value >= entry->new_addr && stack_value < entry->new_end)) {
                log_err("process %d: found patch function 0x%lx on stack\n", pid, entry->new_addr);
                return false;
            }
        }
    }

    return true;
}

/* --- Public interface --- */

struct process_entity *new_process(struct task_struct *task)
{
    struct process_entity *process;

    if (unlikely(!task)) {
        return ERR_PTR(-EINVAL);
    }

    process = kmem_cache_alloc(g_process_cache, GFP_ATOMIC);
    if (!process) {
        return ERR_PTR(-ENOMEM);
    }

    process->task = get_task_struct(task);
    process->tgid = task_tgid_nr(task);

    spin_lock_init(&process->thread_lock);

    INIT_HLIST_NODE(&process->node);
    INIT_LIST_HEAD(&process->pending_node);

    INIT_LIST_HEAD(&process->patch_list);
    process->patch_info = NULL;

    kref_init(&process->kref);

    log_debug("new process %d\n", process->tgid);
    return process;
}

void release_process(struct kref *kref)
{
    struct process_entity *process;
    struct patch_info *patch_info;
    struct patch_info *tmp;

    if (unlikely(!kref)) {
        return;
    }

    process = container_of(kref, struct process_entity, kref);
    log_debug("free process %d\n", process->tgid);

    WARN_ON(spin_is_locked(&process->thread_lock));

    WARN_ON(!hlist_unhashed(&process->node));
    WARN_ON(!list_empty(&process->pending_node));

    list_for_each_entry_safe(patch_info, tmp, &process->patch_list, node) {
        list_del_init(&patch_info->node);
        free_patch_memory(process->task, patch_info);
        free_patch_info(patch_info);
    }
    process->patch_info = NULL;

    put_task_struct(process->task);
    process->task = NULL;
    process->tgid = 0;

    kmem_cache_free(g_process_cache, process);
}

struct patch_info *process_switch_and_get_patch(struct process_entity *process, struct patch_entity *patch)
{
    struct patch_info *patch_info;

    BUG_ON(unlikely(!process || !patch));

    if (likely(process->patch_info && process->patch_info->patch == patch)) {
        return process->patch_info;
    }

    patch_info = find_patch_info_unlocked(process, patch);
    if (unlikely(!patch_info)) {
        return NULL;
    }

    process->patch_info = patch_info;

    return patch_info;
}

unsigned long process_get_jump_addr(struct process_entity *process, unsigned long old_addr)
{
    struct patch_jump_entry *entry;
    unsigned long jump_addr = 0;

    if (unlikely(!process || !process->patch_info)) {
        return 0;
    }

    hash_for_each_possible(process->patch_info->jump_table, entry, node, hash_long(old_addr, PATCH_FUNC_HASH_BITS)) {
        if (entry->old_addr == old_addr) {
            jump_addr = entry->new_addr;
            break;
        }
    }

    return jump_addr;
}

int process_load_patch(struct process_entity *process, struct patch_entity *patch, struct patch_context *ctx)
{
    struct upatch_function *funcs = (struct upatch_function *)ctx->func_shdr->sh_addr;
    size_t func_num = ctx->func_shdr->sh_size / (sizeof (struct upatch_function));

    struct upatch_relocation *relas = (struct upatch_relocation *)ctx->rela_shdr->sh_addr;
    const char *strings = (const char *)ctx->string_shdr->sh_addr;

    const char *func_name;
    size_t i;

    struct patch_info *patch_info;
    struct patch_jump_entry *jump_entry;

    if (unlikely(!process || !patch || !ctx)) {
        return -EINVAL;
    }

    patch_info = kmem_cache_alloc(g_patch_info_cache, GFP_ATOMIC);
    if (!patch_info) {
        return -ENOMEM;
    }

    patch_info->patch = get_patch(patch);
    INIT_LIST_HEAD(&patch_info->node);

    patch_info->text_addr = ctx->layout.base;
    patch_info->text_len = ctx->layout.text_end;

    patch_info->rodata_addr = ctx->layout.base + ctx->layout.text_end;
    patch_info->rodata_len = ctx->layout.ro_after_init_end - ctx->layout.text_end;

    patch_info->jump_min_addr = ULONG_MAX;
    patch_info->jump_max_addr = 0;

    hash_init(patch_info->jump_table);

    for (i = 0; i < func_num; ++i) {
        func_name = strings + relas[i].name.r_addend;

        jump_entry = kmem_cache_alloc(g_jump_entry_cache, GFP_ATOMIC);
        if (!jump_entry) {
            free_patch_info(patch_info);
            return -ENOMEM;
        }

        INIT_HLIST_NODE(&jump_entry->node);
        jump_entry->old_addr = funcs[i].old_addr + ctx->load_bias + ctx->target->load_offset;
        jump_entry->new_addr = funcs[i].new_addr;
        jump_entry->new_end = funcs[i].new_addr + funcs[i].new_size;

        if (patch_info->jump_min_addr > jump_entry->new_addr) {
            patch_info->jump_min_addr = jump_entry->new_addr;
        }
        if (patch_info->jump_max_addr < jump_entry->new_end) {
            patch_info->jump_max_addr = jump_entry->new_end;
        }

        log_debug("process %d: old_addr=0x%08lx, new_addr=0x%08lx, func='%s'\n",
            process->tgid, jump_entry->old_addr, jump_entry->new_addr, func_name);
        hash_add(patch_info->jump_table, &jump_entry->node, hash_long(jump_entry->old_addr, PATCH_FUNC_HASH_BITS));
    }

    list_add(&patch_info->node, &process->patch_list);
    process->patch_info = patch_info;

    return 0;
}

void process_remove_patch(struct process_entity *process, struct patch_entity *patch)
{
    struct patch_info *patch_info;

    patch_info = find_patch_info_unlocked(process, patch);
    if (unlikely(!patch_info)) {
        return;
    }

    list_del_init(&patch_info->node);
    free_patch_memory(process->task, patch_info);
    free_patch_info(patch_info);
}

int process_check_patch_on_stack(struct process_entity *process, struct patch_entity *patch)
{
    struct patch_info *patch_info;

    if (unlikely(!process || !patch)) {
        return -EINVAL;
    }

    patch_info = find_patch_info_unlocked(process, patch);
    if (unlikely(!patch_info)) {
        return 0;
    }

    return check_process_stack(process->task, is_patch_removable, patch_info);
}
