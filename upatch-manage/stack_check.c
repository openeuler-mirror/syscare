// SPDX-License-Identifier: GPL-2.0
/*
 * process stack checking
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

#include "stack_check.h"

#include <linux/mm.h>
#include <linux/highmem.h>
#include <linux/sched/task.h>
#include <linux/sched/task_stack.h>

#include "util.h"

#define STACK_CHECK_MAX_PAGES 32

static int check_stack_pages(pid_t pid, struct page **pages, long count, stack_check_fn check_fn, void *check_ctx)
{
    const void *page;
    long i;
    bool result;

    for (i = 0; i < count; i++) {
        page = kmap_local_page(pages[i]);

        result = check_fn(pid, page, check_ctx);
        kunmap_local(page);

        if (unlikely(!result)) {
            return -EBUSY;
        }
    }

    return 0;
}

static int check_thread_stack(struct task_struct *process, struct task_struct *thread,
    stack_check_fn check_fn, void *check_ctx)
{
    struct mm_struct *mm;

    unsigned long stack_pointer;
    struct vm_area_struct *stack_vma;

    unsigned long stack_start;
    unsigned long stack_end;
    unsigned long stack_page_nr;

    struct page *stack_pages[STACK_CHECK_MAX_PAGES];
    unsigned long stack_addr;
    unsigned long page_nr;
    long page_count;
    int i;

    pid_t tgid = task_tgid_nr(process);
    pid_t pid = task_pid_nr(thread);
    int ret = 0;

    // skip if thread has no mm
    mm = get_task_mm(thread);
    if (unlikely(!mm)) {
        return 0;
    }

    mmap_read_lock(mm);

    stack_pointer = task_pt_regs(thread)->sp;
    if (unlikely(stack_pointer == 0)) {
        goto unlock_mm;
    }

    // find stack vma
    stack_vma = find_vma(mm, stack_pointer);
    if (unlikely(!stack_vma)) {
        goto unlock_mm;
    }

    // check stack vma
    if (!(stack_vma->vm_flags & (VM_READ | VM_WRITE))) {
        goto unlock_mm;
    }

    stack_start = stack_vma->vm_start;
    stack_end = stack_vma->vm_end;
    stack_page_nr = (stack_end - stack_start) >> PAGE_SHIFT;
    if (unlikely(stack_page_nr == 0)) {
        goto unlock_mm;
    }

    log_debug("process %d: thread %d stack at 0x%lx-0x%lx (%lu pages)\n",
        tgid, pid, stack_start, stack_end, stack_page_nr);

    stack_addr = stack_start;
    while (stack_addr < stack_end) {
        page_nr = STACK_CHECK_MAX_PAGES;
        if (stack_addr + (page_nr << PAGE_SHIFT) > stack_end) {
            page_nr = (stack_end - stack_addr) >> PAGE_SHIFT;
        }
        if (page_nr == 0) {
            break;
        }

        page_count = get_user_pages_remote(mm, stack_addr, page_nr, FOLL_GET, stack_pages, NULL);
        if (unlikely(page_count < 0)) {
            ret = page_count;
            log_err("process %d: failed to get stack pages at 0x%lx, ret=%d\n", tgid, stack_addr, ret);
            break;
        } else if (page_count == 0) {
            log_debug("process %d: skipped %lu unmapped pages\n", tgid, page_nr);
            stack_addr += page_nr * PAGE_SIZE;
        } else {
            ret = check_stack_pages(tgid, stack_pages, page_count, check_fn, check_ctx);
            for (i = 0; i < page_count; i++) {
                put_page(stack_pages[i]);
            }
            if (ret) {
                break;
            }
            stack_addr += page_nr * PAGE_SIZE;
        }
    }

unlock_mm:
    mmap_read_unlock(mm);

    mmput(mm);
    return ret;
}

int check_process_stack(struct task_struct *process, stack_check_fn check_fn, void *check_ctx)
{
    struct task_struct *thread;

    int ret = 0;

    if (unlikely(!process || !check_fn || !check_ctx)) {
        return -EINVAL;
    }

    rcu_read_lock();
    for_each_thread(process, thread) {
        ret = check_thread_stack(process, thread, check_fn, check_ctx);
        if (ret) {
            break;
        }
    }
    rcu_read_unlock();

    return ret;
}
