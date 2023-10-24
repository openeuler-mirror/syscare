// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#ifndef _UPATCH_UTILS_H
#define _UPATCH_UTILS_H

void * get_user_params(void __user *ptr, unsigned long len);
void put_user_params(void *ptr);

struct inode* get_path_inode(const char *path);
void put_path_inode(struct inode *inode);

#endif
