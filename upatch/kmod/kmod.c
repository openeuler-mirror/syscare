// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/miscdevice.h>
#include <linux/fs.h>

#include "kmod.h"
#include "patch.h"
#include "compiler.h"

static int upatch_open(struct inode *inode, struct file *file)
{
    return 0;
}

static int upatch_release(struct inode *inode, struct file *file)
{
    return 0;
}

static ssize_t upatch_read(struct file *filp, char __user *ubuf,
			     size_t usize, loff_t *off)
{
    return 0;
}

static ssize_t upatch_write(struct file *filp, const char __user *ubuf,
			      size_t usize, loff_t *off)
{
    return 0;
}

static long upatch_ioctl(struct file *filp, unsigned int cmd, unsigned long arg)
{
    if (_IOC_TYPE(cmd) != UPATCH_IOCTL_MAGIC)
        return -EINVAL;

    switch (cmd) {
    case UPATCH_REGISTER_COMPILER:
    case UPATCH_UNREGISTER_COMPILER:
    case UPATCH_REGISTER_ASSEMBLER:
    case UPATCH_UNREGISTER_ASSEMBLER:
        return handle_compiler_cmd(arg, cmd);
    case UPATCH_INSTALL_PATCH:
        return upatch_attach(arg, cmd);
    // case UPATCH_UNINSTALL_PATCH:
    // case UPATCH_APPLY_PATCH:
    case UPATCH_REMOVE_PATCH:
        return upatch_remove(arg, cmd);
    // case UPATCH_ACTIVE_PATCH:
    // case UPATCH_DEACTIVE_PATCH:
    default:
        return -ENOTTY;
    }

    return 0;
}

static const struct file_operations upatch_ops = {
	.owner		= THIS_MODULE,
	.open		= upatch_open,
	.release	= upatch_release,
	.read		= upatch_read,
	.write		= upatch_write,
	.unlocked_ioctl	= upatch_ioctl,
	.llseek		= no_llseek,
};

static struct miscdevice upatch_dev = {
	.minor	= MISC_DYNAMIC_MINOR,
	.name	= UPATCH_DEV_NAME,
	.fops	= &upatch_ops,
    .mode = 0666,
};

static int __init upatch_init(void)
{
    int ret;

    ret = misc_register(&upatch_dev);
    if (ret) {
        pr_err("register misc device for %s failed\n", UPATCH_DEV_NAME);
        return ret;
    }

    ret = compiler_hack_init();
    if (ret < 0)
        return ret;

    return 0;
}

static void __exit upatch_exit(void)
{
    compiler_hack_exit();
    misc_deregister(&upatch_dev);
}

module_init(upatch_init);
module_exit(upatch_exit);

MODULE_AUTHOR("Longjun Luo (luolongjuna@gmail.com)");
MODULE_AUTHOR("Zongwu Li (lzw32321226@163.com)");
MODULE_DESCRIPTION("kernel module for upatch(live-patch in userspace)");
MODULE_LICENSE("GPL");