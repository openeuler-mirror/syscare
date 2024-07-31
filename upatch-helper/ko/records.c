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

#include "records.h"

#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/string.h>

#include "log.h"
#include "map.h"
#include "utils.h"

int new_uprobe_record(struct uprobe_record **record, uprobe_handler handler,
    const char *path, loff_t offset)
{
    struct inode *inode = NULL;
    struct uprobe_consumer *uc = NULL;
    struct uprobe_record *new_record = NULL;

    if ((record == NULL) || (handler == NULL) ||
        (path == NULL) || (offset == 0)) {
        return -EINVAL;
    }

    inode = path_inode(path);
    if (inode == NULL) {
        pr_err("failed to get file inode, path=%s\n", path);
        return -ENOENT;
    }

    new_record = kzalloc(sizeof(struct uprobe_record), GFP_KERNEL);
    if (new_record == NULL) {
        return -ENOMEM;
    }

    uc = kzalloc(sizeof(struct uprobe_consumer), GFP_KERNEL);
    if (uc == NULL) {
        kfree(new_record);
        return -ENOMEM;
    }
    uc->handler = handler;

    new_record->inode = igrab(inode);
    new_record->offset = offset;
    new_record->uc = uc;

    *record = new_record;
    return 0;
}

void free_uprobe_record(struct uprobe_record *record)
{
    if (record == NULL) {
        return;
    }
    iput(record->inode);
    kfree(record->uc);
    kfree(record);
}

int create_helper_record(struct helper_record **record,
    const char *exec_path, const char *jump_path)
{
    struct helper_record *new_record = NULL;
    struct inode *exec_inode = NULL;
    struct inode *jump_inode = NULL;

    if ((record == NULL) || (exec_path == NULL) || (jump_path == NULL)) {
        return -EINVAL;
    }

    exec_inode = path_inode(exec_path);
    if (exec_inode == NULL) {
        pr_err("failed to get file inode, path=%s\n", exec_path);
        return -ENOENT;
    }

    jump_inode = path_inode(jump_path);
    if (jump_inode == NULL) {
        pr_err("failed to get file inode, path=%s\n", jump_path);
        return -ENOENT;
    }

    new_record = kzalloc(sizeof(struct helper_record), GFP_KERNEL);
    if (record == NULL) {
        return -ENOMEM;
    }

    new_record->exec_inode = igrab(exec_inode);
    new_record->jump_inode = igrab(jump_inode);
    strlcpy(new_record->exec_path, exec_path, PATH_MAX);
    strlcpy(new_record->jump_path, jump_path, PATH_MAX);

    *record = new_record;
    return 0;
}

void free_helper_record(struct helper_record *record)
{
    if (record == NULL) {
        return;
    }

    iput(record->exec_inode);
    iput(record->jump_inode);
    kfree(record);
}

bool find_helper_record(const struct helper_record *record,
    const struct inode *inode)
{
    return (inode_equal(record->exec_inode, inode) ||
        inode_equal(record->jump_inode, inode));
}
