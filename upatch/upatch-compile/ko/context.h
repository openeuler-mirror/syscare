// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_CONTEXT_H
#define _UPATCH_HIJACKER_KO_CONTEXT_H

#include <linux/types.h>

struct map;

int context_init(void);
void context_exit(void);

int build_hijacker_context(const char *path, loff_t offset);
void destroy_hijacker_context(void);
size_t hijacker_context_count(void);

struct map *get_hijacker_map(void);

#endif /* _UPATCH_HIJACKER_KO_CONTEXT_H */
