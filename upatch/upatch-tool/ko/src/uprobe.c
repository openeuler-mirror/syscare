// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#include <linux/uprobes.h>
#include <linux/fs.h>
#include <linux/uaccess.h>
#include <linux/umh.h>
#include <linux/mm.h>

#include "uprobe.h"

monitor_list_t *monitor_list;

int monitor_list_init()
{
	monitor_list = alloc_monitor_list();

	if (!monitor_list) {
		pr_err("upatch-manager: alloc monitor list failed\n");
		return -ENOMEM;
	}

	return 0;
}

void monitor_list_destroy()
{
	if (!monitor_list)
		return;
	free_monitor_list(monitor_list);
	monitor_list = NULL;
}

int upatch_monitor_register(monitor_list_t *mlist, pid_t monitor_pid)
{
	pr_info("upatch-manager: register monitor %d\n", monitor_pid);
	if (!mlist) {
		pr_err("upatch-manager: mlist is not exist\n");
		return -EFAULT;
	}
	return insert_monitor_list(mlist, monitor_pid);
}

void upatch_monitor_deregister(void __user *param, monitor_list_t *mlist)
{
	pid_t pid;

	if (!mlist)
		return;

	if (copy_from_user((void *)&pid, param, sizeof(pid)))
		return;
	pr_info("upatch-manager: deregister monitor %d\n", pid);
	remove_monitor_list(mlist, pid);
}

static int active_patch(uprobe_list_entry_t *entry, char *pid)
{
	int ret;
	char *binary = entry->binary_path;
	char *patch = entry->patch_path;
	char *cmd_path = "/usr/libexec/syscare/upatch-manage";
	char *cmd_envp[] = {"HOME=/", "PATH=/usr/libexec/syscare", NULL};
	char *cmd_argv[] = {cmd_path, "patch", "--pid", pid, "--upatch", patch, "--binary", binary, "-v", NULL};

	ret = call_usermodehelper(cmd_path, cmd_argv, cmd_envp, UMH_WAIT_EXEC);
	pr_info("upatch-manager: %s(%s) patch %s with UMH_WAIT_EXEC ret %d\n", binary, pid, patch, ret);

	return ret;
}

static int active_patches(uprobe_list_t *list, struct inode *inode, pid_t pid)
{
	uprobe_list_entry_t *entry = NULL;
	char pid_str[128] = {0};

	if (!list) {
		goto err_out;
	}

	memset(pid_str, 0, sizeof(pid_str));
	snprintf(pid_str, sizeof(pid_str), "%d", pid);

	list_for_each_entry(entry, &list->list_head, list_node) {
		if (entry->inode == inode) {
			entry->pid = pid;
			if (active_patch(entry, pid_str) < 0)
				goto err_out;
		}
	}
	return 0;
err_out:
	return -1;
}

static int uprobe_handler(struct uprobe_consumer *self, struct pt_regs *regs)
{
	int ret = -EFAULT;
	unsigned long pc;
	struct vm_area_struct *vma_binary = NULL;
	struct file *binary_file = NULL;
	struct inode *inode = NULL;
	monitor_list_entry_t *entry = list_first_entry_or_null(&monitor_list->list_head, monitor_list_entry_t, list_node);

	if (!entry)
		goto out;

	pr_info("upatch-manager: uprobe handler triggered, pid=%d\n", current->pid);

	pc = instruction_pointer(regs);
	pr_debug("patch handler works in 0x%lx \n", pc);

	vma_binary = find_vma(current->mm, pc);
	if (!vma_binary || !vma_binary->vm_file) {
		pr_err("no exe file found for upatch \n");
		goto out;
	}

	binary_file = vma_binary->vm_file;
	inode = file_inode(binary_file);
	ret = active_patches(entry->uprobe_list, inode, current->pid);

	if (ret < 0) {
		goto out;
	}

out:
	return ret;
}

static inline bool uprobe_filter(struct uprobe_consumer *self, enum uprobe_filter_ctx ctx, struct mm_struct *mm)
{
	return true;
}

static struct uprobe_consumer uprobe_consumer = {
	.handler = uprobe_handler,
	.ret_handler = NULL,
	.filter = uprobe_filter,
};

static int __upatch_uprobe_register(struct inode *inode, loff_t offset)
{
	int ret = 0;

	if (!inode) {
		pr_err("upatch-manager: inode is NULL");
		ret = -EINVAL;
		goto err_out;
	}

	pr_info("upatch-manager: register uprobe, inode=%ld, offset=0x%llx\n",
			inode->i_ino, offset);

	ret = uprobe_register(inode, offset, &uprobe_consumer);
	if (ret) {
		pr_err("upatch-manager: failed to register uprobe, inode=%ld, offset=0x%llx\n",
				inode->i_ino, offset);
		goto err_out;
	}

err_out:
	return ret;

}
int upatch_uprobe_register(monitor_list_t *mlist, struct inode *inode, loff_t offset, char *binary_path, char *patch_path)
{
	int ret = -EFAULT;
	pid_t pid = current->pid;
	monitor_list_entry_t *entry = NULL;

	if (!mlist)
		goto err;
	entry = find_monitor_list(mlist, pid);
	if (!entry) {
		goto err;
	}
	if (!entry->uprobe_list) {
		entry->uprobe_list = alloc_uprobe_list();
		if (!entry->uprobe_list)
			goto err;
	}
	ret = insert_uprobe_list(entry->uprobe_list, inode, offset, binary_path, patch_path);
	if (ret < 0)
		goto err;
	ret = __upatch_uprobe_register(inode, offset);
err:
	return ret;
}

int __upatch_uprobe_deregister(struct inode *inode, loff_t offset)
{
	int ret = 0;

	if (!inode) {
		pr_err("upatch-manager: inode is NULL");
		ret = -EINVAL;
		goto err_out;
	}

	pr_info("upatch-manager: deregister uprobe, inode=%ld, offset=0x%llx\n",
			inode->i_ino, offset);

	uprobe_unregister(inode, offset, &uprobe_consumer);

err_out:
	return ret;
}
int upatch_uprobe_deregister(monitor_list_t *mlist, struct inode *inode, loff_t offset, pid_t mpid, elf_request_t *req)
{
	int ret = -EFAULT;
	monitor_list_entry_t *entry = NULL;
	uprobe_list_entry_t *uentry = NULL;

	if (!mlist)
		goto err;
	entry = find_monitor_list(mlist, mpid);
	if (!entry) {
		pr_err("upatch-manager: can not find specific monitor\n");
		goto err;
	}
	if (!entry->uprobe_list) {
		return 0;
	}
	ret = __upatch_uprobe_deregister(inode, offset);
	if (ret < 0) {
		goto err;
	}

	//get pid
	list_for_each_entry(uentry, &entry->uprobe_list->list_head, list_node) {
		if (strncmp(uentry->binary_path, req->elf_path, strlen(req->elf_path)) ||
			strncmp(uentry->patch_path, req->patch_path, strlen(req->patch_path)))
			continue;
		req->monitor_pid = uentry->pid;
		pr_info("upatch-manager: get upatch process pid %d\n", req->monitor_pid);
		break;
	}
	remove_uprobe_list(entry->uprobe_list, inode, offset);
err:
	return ret;
}
