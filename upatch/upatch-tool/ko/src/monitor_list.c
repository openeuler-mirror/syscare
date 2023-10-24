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
#include <linux/slab.h>

#include "monitor_list.h"

monitor_list_t* alloc_monitor_list()
{
	monitor_list_t *list = NULL;

	list = kmalloc(sizeof(monitor_list_t), GFP_KERNEL);
	if (!list) {
		goto err;
	}
	INIT_LIST_HEAD(&list->list_head);
	mutex_init(&list->list_mutex);
err:
	return list;
}

void free_monitor_list(monitor_list_t *list)
{
	monitor_list_entry_t *entry = NULL;
	monitor_list_entry_t *tmp = NULL;

	if (!list) {
		return;
	}

	mutex_lock(&list->list_mutex);
	list_for_each_entry_safe(entry, tmp, &list->list_head, list_node) {
		free_monitor_list_entry(entry);
	}
	mutex_unlock(&list->list_mutex);
	kfree(list);
}

monitor_list_entry_t* alloc_monitor_list_entry(pid_t monitor_pid)
{
	monitor_list_entry_t *entry = NULL;

	entry = kmalloc(sizeof(monitor_list_entry_t), GFP_KERNEL);
	if (!entry) {
		goto err_out;
	}

	INIT_LIST_HEAD(&entry->list_node);
	entry->monitor_pid = monitor_pid;
	entry->uprobe_list = NULL;
	entry->pid_list = NULL;

err_out:
	return entry;
}

void free_monitor_list_entry(monitor_list_entry_t *entry)
{
	if (!entry) {
		return;
	}

	free_pid_list(entry->pid_list);
	free_uprobe_list(entry->uprobe_list);

	kfree(entry);
}

monitor_list_entry_t* find_monitor_list(monitor_list_t *list, pid_t monitor_pid)
{
	monitor_list_entry_t *entry = NULL;
	monitor_list_entry_t *tmp = NULL;

	if (!list) {
		goto err_out;
	}

	list_for_each_entry(tmp, &list->list_head, list_node) {
		if (tmp->monitor_pid == monitor_pid) {
			entry = tmp;
			break;
		}
	}

err_out:
	return entry;
}

int insert_monitor_list(monitor_list_t *list, pid_t monitor_pid)
{
	int ret = 0;
	monitor_list_entry_t *entry = NULL;

	if (!list) {
		pr_err("upatch-manager: monitor list is NULL\n");
		ret = -EINVAL;
		return ret;
	}

	mutex_lock(&list->list_mutex);

	entry = find_monitor_list(list, monitor_pid);
	if (entry) {
		pr_err("upatch-manager: monitor is already exist, monitor_pid=%d\n",
				entry->monitor_pid);
		ret = -EEXIST;
		goto err_out;
	}

	entry = alloc_monitor_list_entry(monitor_pid);
	if (!entry) {
		pr_err("upatch-manager: failed to allocate monitor list entry\n");
		ret = -ENOMEM;
		goto err_out;
	}

	list_add(&entry->list_node, &list->list_head);

err_out:
	mutex_unlock(&list->list_mutex);
	return ret;
}

void remove_monitor_list(monitor_list_t *list, pid_t monitor_pid)
{
	monitor_list_entry_t *entry = NULL;

	if (!list) {
		pr_err("upatch-manager: monitor list is NULL\n");
		return;
	}

	mutex_lock(&list->list_mutex);

	entry = find_monitor_list(list, monitor_pid);
	if (!entry) {
		pr_err("upatch-manager: monitor is not exist, monitor pid=%d\n",
				monitor_pid);
		goto err_out;
	}
	list_del(&entry->list_node);
	free_monitor_list_entry(entry);

err_out:
	mutex_unlock(&list->list_mutex);
}
