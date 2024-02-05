// SPDX-License-Identifier: LGPL-2.1 OR BSD-2-Clause
/*
 * Authors:
 *   RenoSeven <dev@renoseven.net>
 */

#ifndef __UPATCH_HIJACKER_COMMON_H
#define __UPATCH_HIJACKER_COMMON_H

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <linux/limits.h>

static const char *UPATCH_ENV_NAME = "UPATCH_HIJACKER";
static const char *EXEC_SELF_PATH = "/proc/self/exe";
static const char *OUTPUT_FLAG_NAME = "-o";

static char g_filename[PATH_MAX] = { 0 };

static inline char* get_current_exec()
{
    ssize_t path_len = readlink(EXEC_SELF_PATH, (char *)g_filename, PATH_MAX);
    if (path_len == -1) {
        return NULL;
    }
    g_filename[path_len] = '\0';

    return (char *)g_filename;
}

static inline const char* get_hijacker_env()
{
    return getenv(UPATCH_ENV_NAME);
}

static inline int find_output_flag(int argc, char* const argv[])
{
    for (int i = 0; i < argc; i++) {
        if (argv[i] == NULL) {
            break;
        }
        if (strncmp(argv[i], OUTPUT_FLAG_NAME, 2) == 0) {
            return i;
        }
    }

    return -EINVAL;
}

#endif /* __UPATCH_HIJACKER_COMMON_H */
