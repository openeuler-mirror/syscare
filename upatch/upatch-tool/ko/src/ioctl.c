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
#include <linux/uaccess.h>

#include "ioctl.h"
#include "uprobe.h"
#include "utils.h"

int handle_get_pid(void __user *params, monitor_list_t *mlist)
{
	monitor_list_entry_t *entry = NULL;
	pid_list_entry_t *pentry = NULL;

	if (!mlist) {
		pr_err("upatch-manager:monitor not exist\n");
		return -EFAULT;
	}
	entry = list_first_entry_or_null(&mlist->list_head, monitor_list_entry_t, list_node);

	if (!entry) {
                pr_err("upatch-manager:the monitor list is empty\n");
                return -EFAULT;
        }
	if (!entry->pid_list) {
		pr_info("upatch-manager:pid list is empty\n");
		goto out;
        }

	pentry = list_first_entry_or_null(&entry->pid_list->list_head, pid_list_entry_t, list_node);

	if (!pentry) {
		pr_info("upatch-manager:pid list is empty\n");
                goto out;
	}

	copy_to_user(params, (void *)&pentry->pid, sizeof(pentry->pid));
	remove_pid_list_entry(entry->pid_list, pentry);
out:
	return 0;
}

int handle_register_elf(void __user *params, monitor_list_t *mlist)
{
	int ret = 0;
	elf_request_t *req = NULL;
	struct inode* inode = NULL;

	req = (elf_request_t *)get_user_params(params, sizeof(elf_request_t));
	if (!req) {
		pr_err("upatch-manager: failed to get user params");
		goto err_out;
	}

	inode = get_path_inode(req->elf_path);
	if (!inode) {
		pr_err("upatch-manager: failed to get inode of elf \"%s\"", req->elf_path);
		goto err_out;
	}

	pr_info("upatch-manager: process %d register elf \"%s\", offset=0x%llx\n",
			current->pid, req->elf_path, req->offset);

	ret = upatch_uprobe_register(mlist, inode, req->offset, req->elf_path, req->patch_path);
	if (ret) {
		pr_err("upatch-manager: failed to register elf \"%s\", ret=%d\n",
				req->elf_path, ret);
	}

err_out:
	if (inode) {
		put_path_inode(inode);
	}
	if (req) {
		put_user_params(req);
	}
	return ret;
}

int handle_deregister_elf(void __user *params, monitor_list_t *mlist)
{
	int ret = 0;
	elf_request_t *req = NULL;
	struct inode* inode = NULL;

	req = (elf_request_t *)get_user_params(params, sizeof(elf_request_t));
	if (!req) {
		pr_err("upatch-manager: failed to get user params");
		goto err_out;
	}

	inode = get_path_inode(req->elf_path);
	if (!inode) {
		pr_err("upatch-manager: failed to get inode of elf \"%s\"", req->elf_path);
		goto err_out;
	}

	pr_info("upatch-manager: process %d deregister elf \"%s\", offset=0x%llx\n",
			current->pid, req->elf_path, req->offset);

	ret = upatch_uprobe_deregister(mlist, inode, req->offset, req->monitor_pid);
	if (ret) {
		pr_err("upatch-manager: failed to deregister elf \"%s\", ret=%d\n",
				req->elf_path, ret);
	}

err_out:
	if (inode) {
		put_path_inode(inode);
	}
	if (req) {
		put_user_params(req);
	}
	return ret;
}

long handle_ioctl(struct file *filp, unsigned int cmd, unsigned long arg)
{
	int ret = 0;

	if (_IOC_TYPE(cmd) != UPATCH_IOCTL_MAGIC)
		return -EINVAL;

	switch (cmd) {
	case UPATCH_GET_PID:
		ret = handle_get_pid((void __user *)arg, monitor_list);
		break;
	case UPATCH_REGISTER_ELF:
		ret = handle_register_elf((void __user *)arg, monitor_list);
		break;
	case UPATCH_DEREGISTER_ELF:
		ret = handle_deregister_elf((void __user *)arg, monitor_list);
		break;
	case UPATCH_REGISTER_MONITOR:
		ret = upatch_monitor_register(monitor_list, current->pid);
		break;
	case UPATCH_DEREGISTER_MONITOR:
		upatch_monitor_deregister((void __user *)arg, monitor_list);
		break;
	default:
		ret = -EINVAL;
	}

	return ret;
}
