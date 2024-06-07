// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-helper kernel module
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

#ifndef _UPATCH_HELPER_KO_UTILS_H
#define _UPATCH_HELPER_KO_UTILS_H

#include <linux/fs.h>
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/pid_namespace.h>

static inline struct inode* path_inode(const char *path)
{
    struct path kpath;

    if (kern_path(path, LOOKUP_NO_SYMLINKS, &kpath) != 0) {
        return NULL;
    }
    return kpath.dentry->d_inode;
}

static inline bool inode_equal(const struct inode *lhs, const struct inode *rhs)
{
    return (lhs->i_ino == rhs->i_ino);
}

static inline bool ns_equal(const struct pid_namespace *lhs,
    const struct pid_namespace *rhs)
{
    return (lhs->ns.inum == rhs->ns.inum);
}

#endif /* _UPATCH_HELPER_KO_UTILS_H */
