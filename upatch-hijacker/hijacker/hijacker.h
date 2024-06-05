// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * gnu-compiler-hijacker is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
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

static inline char* get_current_exec(void)
{
    ssize_t path_len = readlink(EXEC_SELF_PATH, (char *)g_filename, PATH_MAX);
    if (path_len == -1) {
        return NULL;
    }
    g_filename[path_len] = '\0';

    return (char *)g_filename;
}

static inline const char* get_hijacker_env(void)
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
