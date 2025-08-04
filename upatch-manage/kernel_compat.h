// SPDX-License-Identifier: GPL-2.0
/*
 * when user program hit uprobe trap and go into kernel, load patch into VMA
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

#ifndef _UPATCH_MANAGE_KERNEL_COMPAT_H
#define _UPATCH_MANAGE_KERNEL_COMPAT_H

#include <linux/types.h>
#include <linux/mm_types.h>

#include <linux/version.h>
#include <linux/init.h>

#if LINUX_VERSION_CODE >= KERNEL_VERSION(6,1,0)
    #include <linux/maple_tree.h>
    #define VMA_USE_MAPLE_TREE
#endif

struct upatch_vma_iter {
#ifdef VMA_USE_MAPLE_TREE
    struct ma_state mas;
    unsigned long limit;
#else
    struct vm_area_struct *curr;
#endif
};

void upatch_vma_iter_init(struct upatch_vma_iter *vmi, struct mm_struct *mm);

void upatch_vma_iter_set(struct upatch_vma_iter *vmi, struct vm_area_struct *vma);

struct vm_area_struct *upatch_vma_next(struct upatch_vma_iter *vmi);

struct vm_area_struct *upatch_vma_prev(struct upatch_vma_iter *vmi);

int upatch_mprotect(unsigned long addr, size_t len, unsigned long prot);

int upatch_munmap(struct mm_struct *mm, unsigned long addr, size_t size, struct list_head *uf);

int __init kernel_compat_init(void);
void __exit kernel_compat_exit(void);

#endif // _UPATCH_MANAGE_KERNEL_COMPAT_H
