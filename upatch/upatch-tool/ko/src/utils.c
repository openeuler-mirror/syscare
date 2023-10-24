// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#include <linux/slab.h>
#include <linux/uaccess.h>
#include <linux/namei.h>
#include <linux/fs.h>

#include "utils.h"

void * get_user_params(void __user *ptr, unsigned long len)
{
    int ret;
    void *buf;

    buf = kmalloc(len, GFP_KERNEL);
    if (!buf) {
        pr_err("upatch-manager: kmalloc failed\n");
        return NULL;
    }

    memset(buf, 0, len);

    ret = copy_from_user(buf, ptr, len);
    if (ret) {
        kfree(buf);
        pr_err("upatch-manager: failed to read %ld byte(s) data from %p, ret=%d\n",
            len, ptr, ret);
    }

    return buf;
}

void put_user_params(void *ptr)
{
    if (ptr) {
        kfree(ptr);
    }
}

struct inode* get_path_inode(const char *path)
{
    int ret;
    struct path real_path;

    ret = kern_path(path, LOOKUP_FOLLOW, &real_path);
    if (!ret)
        return igrab(real_path.dentry->d_inode);

    return NULL;
}

void put_path_inode(struct inode *inode)
{
    iput(inode);
}
