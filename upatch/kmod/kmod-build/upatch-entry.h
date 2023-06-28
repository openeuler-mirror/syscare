// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2023 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifndef _UPATCH_ENTRY_H
#define _UPATCH_ENTRY_H

#define UPATCH_ENTRY_MAX_LEN 256

int upatch_get_matched_entry_name(unsigned long prey_ino, const char *name,
    char *buff, unsigned int len);
int upatch_register_entry(unsigned long compiler_ino, const char *dirver_name,
    unsigned long hijacker_ino, const char *hijacker_name);
int upatch_unregister_entry(unsigned long compiler_ino, const char *dirver_name);

#endif /* _UPATCH_ENTRY_H */
