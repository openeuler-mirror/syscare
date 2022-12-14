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
#include <linux/file.h>
#include <linux/fs.h>

#include "kmod.h"
#include "patch.h"
#include "compiler.h"
#include "common.h"

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

static struct file *open_user_path(unsigned long user_addr)
{
    char *elf_path = NULL;
    struct file *elf_file = NULL;

    elf_path = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!elf_path)
        goto out;

    if (copy_para_from_user(user_addr, elf_path, PATH_MAX))
        goto out;

    elf_file = filp_open(elf_path, O_RDONLY, 0);

out:
    if (elf_path)
        kfree(elf_path);
    return elf_file;
}

static int update_status(unsigned long user_addr,
    enum upatch_module_state status)
{
    int ret;
    struct upatch_entity *entity;
    struct file *elf_file = NULL;

    elf_file = open_user_path(user_addr);
    if (!elf_file || IS_ERR(elf_file)) {
        pr_err("open cmd file failed - %d \n", IS_ERR(elf_file));
        ret = -ENOEXEC;
        goto out;
    }

    entity = upatch_entity_get(file_inode(elf_file));
    if (!entity) {
        pr_err("no entity found \n");
        ret = -ENOENT;
        goto out;
    }

    mutex_lock(&entity->entity_status_lock);
    if (entity->set_patch == NULL) {
        pr_err("set status for removed patched is forbidden \n");
        ret = -EPERM;
        goto out_lock;
    } else {
        entity->set_status = status;
    }

    if (entity->set_status == UPATCH_STATE_REMOVED)
        entity->set_patch = NULL;

    ret = 0;
out_lock:
    mutex_unlock(&entity->entity_status_lock);
out:
    if (elf_file && !IS_ERR(elf_file))
        fput(elf_file);
    return ret;
}

static int check_status(unsigned long user_addr)
{
    int ret;
    struct upatch_entity *entity;
    struct file *elf_file = NULL;

    elf_file = open_user_path(user_addr);
    if (!elf_file || IS_ERR(elf_file)) {
        pr_err("open cmd file failed - %d \n", IS_ERR(elf_file));
        ret = -ENOEXEC;
        goto out;
    }

    entity = upatch_entity_get(file_inode(elf_file));
    if (!entity) {
        ret = -ENOENT;
        pr_err("no related entity found \n");
        goto out;
    }     

    mutex_lock(&entity->entity_status_lock);
    ret = entity->set_status;
    mutex_unlock(&entity->entity_status_lock);
out:
    if (elf_file && !IS_ERR(elf_file))
        fput(elf_file);
    return ret;
}

int attach_upatch(unsigned long user_addr)
{
    int ret;
    struct upatch_conmsg conmsg;
    char *binary = NULL;
    char *patch = NULL;

    patch = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!patch) {
        ret = -ENOMEM;
        goto out;
    }

    binary = kmalloc(PATH_MAX, GFP_KERNEL);
    if (!binary) {
        ret = -ENOMEM;
        goto out;
    }

    if (copy_from_user(&conmsg, (const void __user *)user_addr, sizeof(struct upatch_conmsg))) {
        ret = -ENOMEM;
        goto out;
    }

    ret = copy_para_from_user((unsigned long)conmsg.binary, binary, PATH_MAX);
    if (ret)
        goto out;

    ret = copy_para_from_user((unsigned long)conmsg.patch, patch, PATH_MAX);
    if (ret)
        goto out;

    pr_debug("patch %s with %s \n", binary, patch);

    ret = upatch_attach(binary, patch);
    if (ret)
        goto out;

    ret = 0;
out:
    if (binary)
        kfree(binary);
    if (patch)
        kfree(patch);
    return ret;
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
    case UPATCH_ATTACH_PATCH:
        return attach_upatch(arg);
    case UPATCH_ACTIVE_PATCH:
        return update_status(arg, UPATCH_STATE_ACTIVED);
    case UPATCH_DEACTIVE_PATCH:
        return update_status(arg, UPATCH_STATE_RESOLVED);
    case UPATCH_REMOVE_PATCH:
        return update_status(arg, UPATCH_STATE_REMOVED);
    case UPATCH_INFO_PATCH:
        return check_status(arg);
    default:
        return -ENOTTY;
    }

    return 0;
}

static const struct file_operations upatch_ops = {
	.owner		    = THIS_MODULE,
	.open		    = upatch_open,
	.release	    = upatch_release,
	.read		    = upatch_read,
	.write		    = upatch_write,
	.unlocked_ioctl	= upatch_ioctl,
	.llseek		    = no_llseek,
};

static struct miscdevice upatch_dev = {
	.minor	= MISC_DYNAMIC_MINOR,
	.name	= UPATCH_DEV_NAME,
	.fops	= &upatch_ops,
    .mode   = 0666,
};

static int __init upatch_init(void)
{
    int ret;

    ret = compiler_hack_init();
    if (ret < 0)
        return ret;

    ret = misc_register(&upatch_dev);
    if (ret) {
        pr_err("register misc device for %s failed\n", UPATCH_DEV_NAME);
        return ret;
    }

    pr_info("upatch - %s load successfully \n", UPATCH_VERSION);

    return 0;
}

static void __exit upatch_exit(void)
{
    misc_deregister(&upatch_dev);
    compiler_hack_exit();
}

module_init(upatch_init);
module_exit(upatch_exit);

MODULE_AUTHOR("Longjun Luo (luolongjuna@gmail.com)");
MODULE_AUTHOR("Zongwu Li (lzw32321226@163.com)");
MODULE_DESCRIPTION("kernel module for upatch(live-patch in userspace)");
MODULE_LICENSE("GPL");
MODULE_VERSION(UPATCH_VERSION);
