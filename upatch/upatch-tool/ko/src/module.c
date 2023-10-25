// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#include <linux/kernel.h>
#include <linux/module.h>
#include <linux/binfmts.h>
#include <linux/miscdevice.h>
#include <linux/fs.h>

#include "module.h"
#include "ioctl.h"
#include "uprobe.h"
#include "utils.h"
#include "uprobe_list.h"

static const struct file_operations module_fops = {
	.owner		    = THIS_MODULE,
	.unlocked_ioctl = handle_ioctl,
};

static struct miscdevice module_dev = {
	.minor = MISC_DYNAMIC_MINOR,
	.mode  = 0660,
	.name  = UPATCH_MODULE_NAME,
	.fops  = &module_fops,
};

static int __init upatch_manager_init(void)
{
	int ret;

	ret = misc_register(&module_dev);
	if (ret) {
		pr_err("upatch-manager: failed to register misc device %s, ret=%d\n",
				UPATCH_MODULE_NAME, ret);
	}

	pr_info("%s v%s loaded\n", UPATCH_MODULE_NAME, UPATCH_MODULE_VERSION);
	return ret;
}

static void __exit upatch_manager_exit(void)
{
	free_uprobe_list(uprobe_list);
	misc_deregister(&module_dev);

	pr_info("%s v%s removed\n", UPATCH_MODULE_NAME, UPATCH_MODULE_VERSION);
}

module_init(upatch_manager_init);
module_exit(upatch_manager_exit);
MODULE_AUTHOR(UPATCH_MODULE_AUTHOR);
MODULE_DESCRIPTION(UPATCH_MODULE_DESCRIPTION);
MODULE_LICENSE(UPATCH_MODULE_LICENSE);
MODULE_VERSION(UPATCH_MODULE_VERSION);
