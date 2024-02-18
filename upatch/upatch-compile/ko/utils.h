// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_UTILS_H
#define _UPATCH_HIJACKER_KO_UTILS_H

#include <stdbool.h>

#include <linux/fs.h>
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/pid_namespace.h>

static inline struct inode* path_inode(const char *path)
{
    struct path kpath;

    if (kern_path(path, 0, &kpath) != 0) {
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

#endif /* _UPATCH_HIJACKER_KO_UTILS_H */
