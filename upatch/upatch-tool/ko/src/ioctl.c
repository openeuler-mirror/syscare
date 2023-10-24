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

	if (copy_to_user(params, (void *)&pentry->pid, sizeof(pentry->pid)))
		return -EFAULT;

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

	ret = upatch_uprobe_deregister(mlist, inode, req->offset, req->monitor_pid, req);
	if (ret) {
		pr_err("upatch-manager: failed to deregister elf \"%s\", ret=%d\n",
				req->elf_path, ret);
	}
	if (copy_to_user(params, (void *)req, sizeof(*req)))
		return -EFAULT;

err_out:
	if (inode) {
		put_path_inode(inode);
	}
	if (req) {
		put_user_params(req);
	}
	return ret;
}

int handle_active_patch(void __user *params, monitor_list_t *mlist)
{
	return 0;
}

static int unactive_patch(char *binary, char *patch, char *pid)
{
	int ret = 0;
	//char *cmd_path = "/usr/libexec/syscare/upatch-manage";
	//char *cmd_envp[] = {"HOME=/", "PATH=/usr/libexec/syscare:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/root/bin", NULL};
	//char *cmd_argv[] = {cmd_path, "unpatch", "--pid", pid, "--upatch", patch, "--binary", binary, "-v", NULL};

	//ret = call_usermodehelper(cmd_path, cmd_argv, cmd_envp, UMH_WAIT_EXEC);
	pr_info("upatch-manager: %s(%s) unpatch %s with UMH_WAIT_EXEC ret %d\n", binary, pid, patch, ret);

	return ret;
}

int handle_remove_patch(void __user *params, monitor_list_t *mlist)
{
	int ret = 0;
	elf_request_t *req = NULL;
	char pid[128] = {0};

	req = (elf_request_t *)get_user_params(params, sizeof(elf_request_t));
	if (!req) {
		pr_err("upatch-manager: failed to get user params");
		return -EFAULT;
	}
	pr_info("upatch-manager: process %s remove patch \"%s\"\n", req->elf_path, req->patch_path);
	memset(pid, 0, sizeof(pid));
	sprintf(pid, "%d", req->monitor_pid);
	ret = unactive_patch(req->elf_path, req->patch_path, pid);
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
	case UPATCH_ACTIVE_PATCH:
		ret = handle_active_patch((void __user *)arg, monitor_list);
		break;
	case UPATCH_REMOVE_PATCH:
		ret = handle_remove_patch((void __user *)arg, monitor_list);
		break;
	default:
		ret = -EINVAL;
	}

	return ret;
}
