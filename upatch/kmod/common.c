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

/* code from  get_mm_exe_file / get_task_exe_file */
inline struct file *get_binary_file_from_mm(struct mm_struct *mm)
{
	struct file *exe_file;

	rcu_read_lock();
	exe_file = rcu_dereference(mm->exe_file);
	if (exe_file && !get_file_rcu(exe_file))
		exe_file = NULL;
	rcu_read_unlock();
	return exe_file;
}

inline struct file *get_binary_file_from_task(struct task_struct *task)
{
    struct mm_struct *mm;
    struct file *exe_file = NULL;

    task_lock(task);
    mm = task->mm;
    if (mm) {
		if (!(task->flags & PF_KTHREAD))
			exe_file = get_binary_file_from_mm(mm);
	}
    task_unlock(task);
    return exe_file;
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

