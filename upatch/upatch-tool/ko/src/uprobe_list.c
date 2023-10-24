// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#include <linux/types.h>
#include <linux/list.h>
#include <linux/mutex.h>
#include <linux/fs.h>
#include <linux/slab.h>

#include "utils.h"
#include "uprobe.h"
#include "uprobe_list.h"

static inline uprobe_list_entry_t* alloc_uprobe_list_entry(struct inode *inode, loff_t offset, char *binary_path, char *patch_path)
{
	uprobe_list_entry_t *entry = NULL;

	entry = kmalloc(sizeof(uprobe_list_entry_t), GFP_KERNEL);
	if (!entry) {
		goto err_out;
	}

	memset((void *)entry, 0, sizeof(uprobe_list_entry_t));
	INIT_LIST_HEAD(&entry->list_node);
	entry->inode = inode;
	entry->offset = offset;
	memcpy((void *)entry->binary_path, (void *)binary_path, strlen(binary_path));
	memcpy((void *)entry->patch_path, (void *)patch_path, strlen(patch_path));

err_out:
	return entry;
}

static inline void free_uprobe_list_entry(uprobe_list_entry_t *entry)
{
	if (!entry) {
		return;
	}
	put_path_inode(entry->inode);
	kfree(entry);
}

uprobe_list_t* alloc_uprobe_list()
{
	uprobe_list_t *list = NULL;

	list = kmalloc(sizeof(uprobe_list_t), GFP_KERNEL);
	if (!list) {
		goto err_out;
	}
	INIT_LIST_HEAD(&list->list_head);
	mutex_init(&list->list_mutex);

err_out:
	return list;
}

void free_uprobe_list(uprobe_list_t *list)
{
	uprobe_list_entry_t *entry = NULL;
	uprobe_list_entry_t *tmp = NULL;

	if (!list) {
		return;
	}

	mutex_lock(&list->list_mutex);

	list_for_each_entry_safe(entry, tmp, &list->list_head, list_node) {
		__upatch_uprobe_deregister(entry->inode, entry->offset);
		free_uprobe_list_entry(entry);
	}

	mutex_unlock(&list->list_mutex);
	kfree(list);
}

uprobe_list_entry_t* find_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset)
{
	uprobe_list_entry_t *entry = NULL;
	uprobe_list_entry_t *tmp = NULL;

	if (!list) {
		goto err_out;
	}

	list_for_each_entry(tmp, &list->list_head, list_node) {
		if (tmp->inode == inode && tmp->offset == offset) {
			entry = tmp;
			break;
		}
	}

err_out:
	return entry;
}

int insert_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset, char *binary_path, char *patch_path)
{
	int ret = 0;
	uprobe_list_entry_t *entry = NULL;

	if (!list) {
		pr_err("upatch-manager: uprobe list is NULL\n");
		ret = -EINVAL;
		return ret;
	}

	mutex_lock(&list->list_mutex);

	entry = find_uprobe_list(list, inode, offset);
	if (entry) {
		pr_err("upatch-manager: uprobe is already exist, inode=%ld, offset=0x%llx\n",
				inode->i_ino, offset);
		ret = -EEXIST;
		goto err_out;
	}

	entry = alloc_uprobe_list_entry(inode, offset, binary_path, patch_path);
	if (!entry) {
		pr_err("upatch-manager: failed to allocate uprobe list entry\n");
		ret = -ENOMEM;
		goto err_out;
	}
	


	list_add(&entry->list_node, &list->list_head);

err_out:
	mutex_unlock(&list->list_mutex);
	return ret;
}

void remove_uprobe_list(uprobe_list_t *list, struct inode *inode, loff_t offset)
{
	uprobe_list_entry_t *entry = NULL;

	if (!list) {
		pr_err("upatch-manager: uprobe list is NULL\n");
		return;
	}

	mutex_lock(&list->list_mutex);

	entry = find_uprobe_list(list, inode, offset);
	if (!entry) {
		pr_err("upatch-manager: uprobe is not exist, inode=%ld, offset=0x%llx\n",
				inode->i_ino, offset);
		goto err_out;
	}

	list_del(&entry->list_node);
	free_uprobe_list_entry(entry);

err_out:
	mutex_unlock(&list->list_mutex);
}
