// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/slab.h>
#include <linux/elf.h>
#include <linux/fs.h>
#include <linux/mm.h>

#include "common.h"

/* Common used tool functions */
inline int copy_para_from_user(unsigned long addr, char *buf, size_t buf_len)
{
    size_t len;

    if (!buf || addr == 0)
        return -EINVAL;

    len = strnlen_user((void __user *)addr, MAX_ARG_STRLEN);
    if (len > buf_len)
        return -EOVERFLOW;

    if (copy_from_user(buf, (void __user *)addr, len))
        return -ENOMEM;

    return 0;
}

struct file *get_binary_file_from_addr(struct task_struct *task, unsigned long addr)
{
    struct vm_area_struct *vma = NULL;

    vma = find_vma(task->mm, addr);
    if (!vma)
        return NULL;

    if (!vma->vm_file)
        return NULL;

    return vma->vm_file;
}

/* TODO: handle read from inode, need handle lock here */
struct file *d_open_inode(struct inode *inode)
{
    struct  dentry *alias;
    // unsigned long flags;
    char *name = __getname(), *p;
    struct  file *d_file = NULL;

    if (hlist_empty(&inode->i_dentry))
	    return NULL;

    // raw_spin_lock_bh(&inode->i_lock, flags);
    alias = hlist_entry(inode->i_dentry.first, struct dentry, d_u.d_alias);
    p = dentry_path_raw(alias, name, PATH_MAX);
    if (IS_ERR(p))
        goto out_unlock;

    d_file = filp_open(p, O_RDWR, 0);
out_unlock:
    __putname(name);
    // spin_unlock_irqrestore(&inode->i_lock, flags);
    return d_file;
}
