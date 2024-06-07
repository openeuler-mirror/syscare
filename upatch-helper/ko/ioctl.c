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

#include "ioctl.h"

#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/slab.h>
#include <linux/types.h>
#include <linux/uaccess.h>

#include "log.h"
#include "map.h"
#include "records.h"
#include "context.h"
#include "utils.h"

static const struct file_operations HELPER_DEV_FOPS = {
    .owner = THIS_MODULE,
    .unlocked_ioctl = handle_ioctl,
};

static struct miscdevice g_helper_dev = {
    .minor = MISC_DYNAMIC_MINOR,
    .mode = UPATCH_HELPER_DEV_MODE,
    .name = UPATCH_HELPER_DEV_NAME,
    .fops = &HELPER_DEV_FOPS,
};

static inline int handle_enable_helper(void __user *arg)
{
    int ret = 0;
    upatch_enable_request_t *msg = NULL;

    msg = kzalloc(sizeof(upatch_enable_request_t), GFP_KERNEL);
    if (msg == NULL) {
        pr_err("failed to alloc message\n");
        return -ENOMEM;
    }

    ret = copy_from_user(msg, arg, sizeof(upatch_enable_request_t));
    if (ret != 0) {
        pr_err("failed to copy message from user space\n");
        kfree(msg);
        return -EFAULT;
    }

    pr_debug("enable helper, path=%s, offset=0x%llx\n", msg->path, msg->offset);
    ret = build_helper_context(msg->path, msg->offset);
    if (ret != 0) {
        pr_err("failed to build helper context, ret=%d\n", ret);
        kfree(msg);
    }

    kfree(msg);
    return 0;
}

static inline void handle_disable_helper(void)
{
    pr_debug("disable helper\n");
    destroy_helper_context();
}

static inline int handle_register_helper(void __user *arg)
{
    upatch_register_request_t *msg = NULL;
    struct map *helper_map = get_helper_map();
    struct helper_record *record = NULL;
    int ret = 0;

    if (helper_map == NULL) {
        pr_err("failed to get helper map\n");
        return -EFAULT;
    }

    msg = kzalloc(sizeof(upatch_register_request_t), GFP_KERNEL);
    if (msg == NULL) {
        pr_err("failed to alloc message\n");
        return -ENOMEM;
    }

    ret = copy_from_user(msg, arg, sizeof(upatch_register_request_t));
    if (ret != 0) {
        pr_err("failed to copy message from user space\n");
        kfree(msg);
        return -EFAULT;
    }

    ret = create_helper_record(&record, msg->exec_path, msg->jump_path);
    if (ret != 0) {
        pr_err("failed to create helper record [%s -> %s], ret=%d\n",
            msg->exec_path, msg->jump_path, ret);
        kfree(msg);
        return ret;
    }

    pr_debug("register helper, inode=%lu, addr=0x%lx\n",
        record->exec_inode->i_ino, (unsigned long)record);
    ret = map_insert(get_helper_map(), record);
    if (ret != 0) {
        pr_err("failed to register helper record [%s -> %s], ret=%d\n",
            msg->exec_path, msg->jump_path, ret);
        free_helper_record(record);
        kfree(msg);
        return ret;
    }

    kfree(msg);
    return 0;
}

static inline int handle_unregister_helper(void __user *arg)
{
    upatch_register_request_t *msg = NULL;
    struct map *helper_map = get_helper_map();
    struct inode *inode = NULL;

    int ret = 0;

    if (helper_map == NULL) {
        pr_err("failed to get helper map\n");
        return -EFAULT;
    }


    msg = kzalloc(sizeof(upatch_register_request_t), GFP_KERNEL);
    if (msg == NULL) {
        pr_err("failed to alloc message\n");
        return -ENOMEM;
    }

    ret = copy_from_user(msg, arg, sizeof(upatch_register_request_t));
    if (ret != 0) {
        pr_err("failed to copy message from user space\n");
        kfree(msg);
        return -EFAULT;
    }

    inode = path_inode(msg->exec_path);
    if (inode == NULL) {
        pr_err("failed to get file inode, path=%s\n", msg->exec_path);
        kfree(msg);
        return -ENOENT;
    }

    pr_debug("remove helper, inode=%lu\n", inode->i_ino);
    map_remove(helper_map, inode);

    kfree(msg);
    return 0;
}

int ioctl_init(void)
{
    int ret = 0;

    ret = misc_register(&g_helper_dev);
    if (ret != 0) {
        pr_err("failed to register misc device, ret=%d\n", ret);
    }

    return ret;
}

void ioctl_exit(void)
{
    misc_deregister(&g_helper_dev);
}

long handle_ioctl(struct file *file,
    unsigned int cmd, unsigned long arg)
{
    int ret = 0;

    if (_IOC_TYPE(cmd) != UPATCH_HELPER_IOC_MAGIC) {
        pr_info("invalid command\n");
        return -EBADMSG;
    }

    switch (cmd) {
    case UPATCH_HELPER_ENABLE:
        ret = handle_enable_helper((void __user *)arg);
        break;
    case UPATCH_HELPER_DISABLE:
        handle_disable_helper();
        break;
    case UPATCH_HELPER_REGISTER:
        ret = handle_register_helper((void __user *)arg);
        break;
    case UPATCH_HELPER_UNREGISTER:
        ret = handle_unregister_helper((void __user *)arg);
        break;
    default:
        ret = -EBADMSG;
        break;
    }

    return (long)ret;
}
