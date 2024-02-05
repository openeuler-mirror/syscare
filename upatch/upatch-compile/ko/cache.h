// SPDX-License-Identifier: GPL-2.0
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef _UPATCH_HIJACKER_KO_CACHE_H
#define _UPATCH_HIJACKER_KO_CACHE_H

int cache_init(void);
void cache_exit(void);

char *path_buf_alloc(void);
void path_buf_free(char *buff);

#endif /* _UPATCH_HIJACKER_KO_CACHE_H */
