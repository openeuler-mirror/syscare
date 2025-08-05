// SPDX-License-Identifier: GPL-2.0
/*
 * upatch_manage kernel module
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

#include "kernel_compat.h"

#include <linux/compiler.h>
#include <linux/types.h>
#include <linux/err.h>
#include <linux/sched.h>
#include <linux/mm.h>
#include <linux/kprobes.h>
#include <linux/module.h>
#include <linux/lockdep.h>
#include <linux/version.h>

#if LINUX_VERSION_CODE >= KERNEL_VERSION(6,1,0)
    #include <linux/maple_tree.h>
    #define VMA_USE_MAPLE_TREE
#endif

#include "util.h"

__always_inline void upatch_vma_iter_init(struct upatch_vma_iter *vmi, struct mm_struct *mm)
{
    if (unlikely(!vmi)) {
        return;
    }
    *vmi = (struct upatch_vma_iter){0};

    if (unlikely(!mm)) {
        return;
    }
    lockdep_assert_held(&mm->mmap_lock);

#ifdef VMA_USE_MAPLE_TREE
    mas_init(&vmi->mas, &mm->mm_mt, 0);
    vmi->limit = mm->task_size;
#else
    vmi->curr = mm->mmap;
#endif
}

__always_inline void upatch_vma_iter_set(struct upatch_vma_iter *vmi, struct vm_area_struct *vma)
{
    if (unlikely(!vmi)) {
        return;
    }
    *vmi = (struct upatch_vma_iter){0};

    if (unlikely(!vma || !vma->vm_mm)) {
        return;
    }
    lockdep_assert_held(&vma->vm_mm->mmap_lock);

#ifdef VMA_USE_MAPLE_TREE
    mas_init(&vmi->mas, &vma->vm_mm->mm_mt, 0);
    mas_set(&vmi->mas, vma->vm_end);
    vmi->limit = vma->vm_mm->task_size;
#else
    vmi->curr = vma;
#endif
}

__always_inline struct vm_area_struct *upatch_vma_next(struct upatch_vma_iter *vmi)
{
    if (unlikely(!vmi)) {
        return NULL;
    }
#ifdef VMA_USE_MAPLE_TREE
    return mas_next(&vmi->mas, vmi->limit);
#else
    if (unlikely(!vmi->curr)) {
        return NULL;
    }
    struct vm_area_struct *vma = vmi->curr;
    vmi->curr = vma->vm_next;
    return vma;
#endif
}

__always_inline struct vm_area_struct *upatch_vma_prev(struct upatch_vma_iter *vmi)
{
    if (unlikely(!vmi)) {
        return NULL;
    }
#ifdef VMA_USE_MAPLE_TREE
    return mas_prev(&vmi->mas, 0);
#else
    if (unlikely(!vmi->curr)) {
        return NULL;
    }
    struct vm_area_struct *vma = vmi->curr;
    vmi->curr = vma->vm_prev;
    return vma;
#endif
}

typedef long (*do_mprotect_pkey_fn)(
    unsigned long start,
    size_t len,
    unsigned long prot,
    int pkey
);

static const char *mprotect_symbol_names[] = {
    "do_mprotect_pkey",
    "do_mprotect_pkey.constprop.0",
    "do_mprotect_pkey.constprop.1",
    "do_mprotect_pkey.constprop.2",
    NULL
};
static do_mprotect_pkey_fn do_mprotect_pkey = NULL;

static void *get_kernel_symbol(const char *symbol_name)
{
    struct kprobe kp = {
        .symbol_name = symbol_name,
    };
    void *addr;
    int ret;

    if (!symbol_name) {
        return ERR_PTR(-EINVAL);
    }

    ret = register_kprobe(&kp);
    if (ret < 0) {
        return ERR_PTR(ret);
    }

    addr = (void *)kp.addr;
    if (!addr) {
        log_err("kernel symbol '%s' is NULL\n", symbol_name);
        unregister_kprobe(&kp);
        return ERR_PTR(-EFAULT);
    }

    unregister_kprobe(&kp);
    return addr;
}

int upatch_mprotect(unsigned long addr, size_t len, unsigned long prot)
{
    if (!do_mprotect_pkey || IS_ERR(do_mprotect_pkey)) {
        return -ENOSYS;
    }
    return do_mprotect_pkey(addr, len, prot, -1);
}

int __init kernel_compat_init(void)
{
    int ret;
    const char *symbol_name = NULL;
    void *addr = NULL;

    for (const char **name = mprotect_symbol_names; *name; name++) {
        addr = get_kernel_symbol(*name);
        if (IS_ERR(addr)) {
            ret = PTR_ERR(addr);
            continue;
        }
        symbol_name = *name;
        do_mprotect_pkey = addr;
        break;
    }

    if (!symbol_name) {
        ret = PTR_ERR(addr);
        log_err("cannot find kernel symbol '%s', ret=%d\n", mprotect_symbol_names[0], ret);
        return ret;
    }

    return 0;
}

void __exit kernel_compat_exit(void)
{
    return;
}
