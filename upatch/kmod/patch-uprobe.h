// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_UPROBE_H
#define _UPATCH_UPROBE_H

#include <linux/uprobes.h>


/* Common used functions for uprobe */

/* We don't utilize filter now */
static inline bool uprobe_default_filter(struct uprobe_consumer *self,
    enum uprobe_filter_ctx ctx, struct mm_struct *mm)
{
    return true;
}

elf_addr_t calculate_load_address(struct file *, bool);

#endif /* _UPATCH_UPROBE_H */


