// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
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

static const struct file_operations HIJACKER_DEV_FOPS = {
    .owner = THIS_MODULE,
    .unlocked_ioctl = handle_ioctl,
};

static struct miscdevice g_hijacker_dev = {
    .minor = MISC_DYNAMIC_MINOR,
    .mode = UPATCH_HIJACKER_DEV_MODE,
    .name = UPATCH_HIJACKER_DEV_NAME,
    .fops = &HIJACKER_DEV_FOPS,
};

static inline int handle_enable_hijacker(void __user *arg)
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

    pr_debug("enable hijacker, path=%s, offset=0x%llx\n", msg->path, msg->offset);
    ret = build_hijacker_context(msg->path, msg->offset);
    if (ret != 0) {
        pr_err("failed to build hijacker context, ret=%d\n", ret);
        kfree(msg);
    }

    kfree(msg);
    return 0;
}

static inline void handle_disable_hijacker(void)
{
    pr_debug("disable hijacker\n");
    destroy_hijacker_context();
}

static inline int handle_register_hijacker(void __user *arg)
{
    upatch_register_request_t *msg = NULL;
    struct map *hijacker_map = get_hijacker_map();
    struct hijacker_record *record = NULL;
    int ret = 0;

    if (hijacker_map == NULL) {
        pr_err("failed to get hijacker map\n");
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

    ret = create_hijacker_record(&record, msg->exec_path, msg->jump_path);
    if (ret != 0) {
        pr_err("failed to create hijacker record [%s -> %s], ret=%d\n",
            msg->exec_path, msg->jump_path, ret);
        kfree(msg);
        return ret;
    }

    pr_debug("register hijacker, inode=%lu, addr=0x%lx\n",
        record->exec_inode->i_ino, (unsigned long)record);
    ret = map_insert(get_hijacker_map(), record);
    if (ret != 0) {
        pr_err("failed to register hijacker record [%s -> %s], ret=%d\n",
            msg->exec_path, msg->jump_path, ret);
        free_hijacker_record(record);
        kfree(msg);
        return ret;
    }

    kfree(msg);
    return 0;
}

static inline int handle_unregister_hijacker(void __user *arg)
{
    upatch_register_request_t *msg = NULL;
    struct map *hijacker_map = get_hijacker_map();
    struct inode *inode = NULL;

    int ret = 0;

    if (hijacker_map == NULL) {
        pr_err("failed to get hijacker map\n");
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

    pr_debug("remove hijacker, inode=%lu\n", inode->i_ino);
    map_remove(hijacker_map, inode);

    kfree(msg);
    return 0;
}

int ioctl_init(void)
{
    int ret = 0;

    ret = misc_register(&g_hijacker_dev);
    if (ret != 0) {
        pr_err("failed to register misc device, ret=%d\n", ret);
    }

    return ret;
}

void ioctl_exit(void)
{
    misc_deregister(&g_hijacker_dev);
}

long handle_ioctl(struct file *file,
    unsigned int cmd, unsigned long arg)
{
    int ret = 0;

    if (_IOC_TYPE(cmd) != UPATCH_HIJACKER_IOC_MAGIC) {
        pr_info("invalid command\n");
        return -EBADMSG;
    }

    switch (cmd) {
    case UPATCH_HIJACKER_ENABLE:
        ret = handle_enable_hijacker((void __user *)arg);
        break;
    case UPATCH_HIJACKER_DISABLE:
        handle_disable_hijacker();
        break;
    case UPATCH_HIJACKER_REGISTER:
        ret = handle_register_hijacker((void __user *)arg);
        break;
    case UPATCH_HIJACKER_UNREGISTER:
        ret = handle_unregister_hijacker((void __user *)arg);
        break;
    default:
        ret = -EBADMSG;
        break;
    }

    return (long)ret;
}
