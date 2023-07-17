#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/file.h>
#include <linux/binfmts.h>
#include <linux/kprobes.h>
#include <linux/miscdevice.h>

#include "upatch-entry.h"
#include "ioctl-build.h"

/* code from fs/namei.c */
#define EMBEDDED_NAME_MAX	(PATH_MAX - offsetof(struct filename, iname))

static int hijacker_jump(struct pt_regs *ctx, const char *driver_name)
{
    struct filename *filename = (void *)regs_get_kernel_argument(ctx, 1);
    memcpy((char *)filename->name, driver_name, strlen(driver_name) + 1);
    return 0;
}

static int jump_hijacker(struct pt_regs *ctx, const char *hijacker_name)
{
    struct filename *filename = (void *)regs_get_kernel_argument(ctx, 1);
    memcpy((char *)filename->name, hijacker_name, strlen(hijacker_name) + 1);
    return 0;
}

static int __kprobes hijack_execve_pre(struct kprobe *p, struct pt_regs *ctx)
{
    unsigned int prey_ino = current->mm->exe_file->f_inode->i_ino;
    struct filename *filename = (void *)regs_get_kernel_argument(ctx, 1);
    char jumper_name[UPATCH_ENTRY_MAX_LEN];

    if (!upatch_get_matched_entry_name(prey_ino, NULL, (char *)&jumper_name, UPATCH_ENTRY_MAX_LEN))
        return hijacker_jump(ctx, (char *)&jumper_name);
    else if (!upatch_get_matched_entry_name(prey_ino, filename->name, (char *)&jumper_name, UPATCH_ENTRY_MAX_LEN))
        return jump_hijacker(ctx, (char *)&jumper_name);
    return 0;
}

static struct kprobe kp = {
    .pre_handler = hijack_execve_pre,
    .post_handler = NULL,
};

inline int copy_para_from_user(unsigned long addr, char *buf, size_t buf_len)
{
    size_t len;

    if (!buf || addr == 0)
        return -EINVAL;

    len = strnlen_user((void __user *)addr, MAX_ARG_STRLEN);
    if (len < 0 || len > buf_len)
        return -EOVERFLOW;

    if (copy_from_user(buf, (void __user *)addr, len))
        return -ENOMEM;

    return 0;
}

static int hijacker_entry(unsigned int cmd, unsigned long arg)
{
    int ret;
    struct upatch_hijack_msg msg;
    char driver_name[UPATCH_ENTRY_MAX_LEN];
    char hijacker_name[UPATCH_ENTRY_MAX_LEN];

    ret = -ENOMEM;
    if (copy_from_user(&msg, (const void __user *)arg, sizeof(struct upatch_hijack_msg)))
        goto out;

    ret = copy_para_from_user((unsigned long)msg.driver_name,
        driver_name, UPATCH_ENTRY_MAX_LEN);
    if (ret)
        goto out;

    if (cmd == UPATCH_REGISTER_ENTRY)
        ret = copy_para_from_user((unsigned long)msg.hijacker_name,
            hijacker_name, UPATCH_ENTRY_MAX_LEN);
    if (ret)
        goto out;
    
    if (cmd == UPATCH_REGISTER_ENTRY)
        ret = upatch_register_entry(msg.compiler_ino, (char *)driver_name,
        msg.hijacker_ino, (char *)hijacker_name);
    else
        ret = upatch_unregister_entry(msg.compiler_ino, (char *)driver_name);

out:
    if (ret)
        pr_err("hijacker entry failed - %d \n", ret);
    return ret;
}

static long upatch_build_ioctl(struct file *filp, unsigned int cmd, unsigned long arg)
{
    if (_IOC_TYPE(cmd) != UPATCH_IOCTL_MAGIC)
        return -EINVAL;

    switch (cmd) {
    case UPATCH_REGISTER_ENTRY:
    case UPATCH_UNREGISTER_ENTRY:
        return hijacker_entry(cmd, arg);
    default:
        return -ENOTTY;
    }
    return 0;
}

static const struct file_operations dev_ops = {
    .owner		    = THIS_MODULE,
    .unlocked_ioctl = upatch_build_ioctl,
};

static struct miscdevice upatch_dev = {
    .minor = MISC_DYNAMIC_MINOR,
    .mode  = 0660,
    .name  = UPATCH_BUILD_DEV_NAME,
    .fops  = &dev_ops,
};

static int hijacker_setup(void)
{
    int ret;

    if (UPATCH_ENTRY_MAX_LEN >= EMBEDDED_NAME_MAX) {
        pr_err("overflowed name for upatch-hijacker \n");
        return -EINVAL;
    }

    ret = misc_register(&upatch_dev);
    if (ret) {
        pr_err("register misc device for %s failed\n", UPATCH_BUILD_DEV_NAME);
        return ret;
    }

    return 0;
}

static int __init upatch_build_module_init(void)
{
    int ret;

    ret = hijacker_setup();
    if (ret < 0)
        goto out;

    kp.symbol_name = "do_execveat_common.isra.0";
    ret = register_kprobe(&kp);
    if (ret == -ENOENT) {
        /* If not found, try another name */
        kp.symbol_name = "do_execveat_common";
        ret = register_kprobe(&kp);
    }

    if (ret < 0) {
        pr_err("register kprobe for execve failed - %d \n", ret);
        goto out;
    }
out: 
    return ret;
}

static void __exit upatch_build_module_exit(void)
{
    misc_deregister(&upatch_dev);
    unregister_kprobe(&kp);
}

module_init(upatch_build_module_init);
module_exit(upatch_build_module_exit);
MODULE_AUTHOR("Longjun Luo (luolongjuna@gmail.com)");
MODULE_DESCRIPTION("kernel module for upatch-build (live-patch in userspace)");
MODULE_LICENSE("GPL");
MODULE_VERSION("1.0");
