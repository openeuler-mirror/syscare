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

#include "pid_list.h"

static inline pid_list_entry_t* alloc_pid_list_entry(pid_t pid)
{
    pid_list_entry_t *entry = NULL;

    entry = kmalloc(sizeof(pid_list_entry_t), GFP_KERNEL);
    if (!entry) {
        goto err_out;
    }
    INIT_LIST_HEAD(&entry->list_node);
    entry->pid = pid;

err_out:
    return entry;
}

static inline void free_pid_list_entry(pid_list_entry_t *entry)
{
    if (!entry) {
        return;
    }
    kfree(entry);
}

pid_list_t* alloc_pid_list()
{
    pid_list_t *list = NULL;

    list = kmalloc(sizeof(pid_list_t), GFP_KERNEL);
    if (!list) {
        goto err_out;
    }
    INIT_LIST_HEAD(&list->list_head);
    mutex_init(&list->list_mutex);

err_out:
    return list;
}

void free_pid_list(pid_list_t *list)
{
    pid_list_entry_t *entry = NULL;
    pid_list_entry_t *tmp = NULL;

    if (!list) {
        return;
    }

    mutex_lock(&list->list_mutex);

    list_for_each_entry_safe(entry, tmp, &list->list_head, list_node) {
	list_del(&entry->list_node);
        free_pid_list_entry(entry);
    }

    mutex_unlock(&list->list_mutex);
    kfree(list);
}

pid_list_entry_t* find_pid_list(pid_list_t *list, pid_t pid)
{
    pid_list_entry_t *entry = NULL;

    list_for_each_entry(entry, &list->list_head, list_node) {
        if (entry->pid == pid) {
            return entry;
        }
    }

    return NULL;
}

int insert_pid_list(pid_list_t *list, pid_t pid)
{
    int ret = 0;
    pid_list_entry_t *entry = NULL;

    if (!list) {
        pr_err("upatch-manager: pid list is NULL\n");
        ret = -EINVAL;
        goto err_out;
    }

    mutex_lock(&list->list_mutex);

    entry = find_pid_list(list, pid);
    if (entry) {
        pr_err("upatch-manager: pid %d is already exist\n", pid);
        ret = -EEXIST;
        goto err_out;
    }

    entry = alloc_pid_list_entry(pid);
    if (!entry) {
        pr_err("upatch-manager: failed to allocate pid list entry\n");
        ret = -ENOMEM;
        goto err_out;
    }

    list_add(&entry->list_node, &list->list_head);

err_out:
    mutex_unlock(&list->list_mutex);
    return ret;
}

void remove_pid_list(pid_list_t *list, pid_t pid)
{
    pid_list_entry_t *entry = NULL;

    if (!list) {
        pr_err("upatch-manager: pid list is NULL\n");
        goto err_out;
    }

    mutex_lock(&list->list_mutex);

    entry = find_pid_list(list, entry->pid);
    if (!entry) {
        pr_err("upatch-manager: pid %d is not exist\n", pid);
        goto err_out;
    }

    list_del(&entry->list_node);
    free_pid_list_entry(entry);

err_out:
    mutex_unlock(&list->list_mutex);
}

pid_list_entry_t *get_pid_list_first_entry(pid_list_t *list)
{
    return list_first_entry_or_null(&list->list_head, pid_list_entry_t, list_node);
}

void remove_pid_list_entry(pid_list_t *list, pid_list_entry_t *entry)
{
    mutex_lock(&list->list_mutex);

    list_del(&entry->list_node);
    free_pid_list_entry(entry);
    mutex_unlock(&list->list_mutex);
}
