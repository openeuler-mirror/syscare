// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 *
 */

#ifndef _UPATCH_UPROBE_H
#define _UPATCH_UPROBE_H

#include "ioctl.h"

int __upatch_uprobe_deregister(struct inode *inode, loff_t offset);
int upatch_uprobe_register(struct inode *inode, loff_t offset, char *binary_path, char *patch_path);
int upatch_uprobe_deregister(struct inode *inode, loff_t offset, struct elf_request *req);

#endif
