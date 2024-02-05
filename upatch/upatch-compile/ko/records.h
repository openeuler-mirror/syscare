// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_ENTITY_H
#define _UPATCH_HIJACKER_KO_ENTITY_H

#include <linux/types.h>
#include <linux/limits.h>
#include <stdbool.h>

struct inode;
struct uprobe_consumer;
struct pt_regs;

typedef int (*uprobe_handler)(struct uprobe_consumer *uc, struct pt_regs *regs);

struct uprobe_record {
    struct inode *inode;
    loff_t offset;
    struct uprobe_consumer *uc;
};

struct hijacker_record {
    struct inode *exec_inode;
    struct inode *jump_inode;
    char exec_path[PATH_MAX];
    char jump_path[PATH_MAX];
};

int new_uprobe_record(struct uprobe_record **record,
    uprobe_handler handler, const char *path, loff_t offset);
void free_uprobe_record(struct uprobe_record *record);

int create_hijacker_record(struct hijacker_record **record,
    const char *exec_path, const char *jump_path);
void free_hijacker_record(struct hijacker_record *record);
bool find_hijacker_record(const struct hijacker_record *record,
    const struct inode *inode);

#endif /* _UPATCH_HIJACKER_KO_ENTITY_H */
