// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * gnu-compiler-helper is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

#include <stdio.h>

#include "helper.h"

static char* APPEND_ARGS[] = {
    "-gdwarf", /* obatain debug information */
    "-ffunction-sections",
    "-fdata-sections",
    "-frecord-gcc-switches",
};
static const int APPEND_ARG_LEN = (int)(sizeof(APPEND_ARGS) / sizeof(char *));

/*
 * The whole part:
 * 1. Someone called execve() to run a compiler (inode).
 * 2. If the inode was registered, under layer would rewrite argv[0] to helper path.
 * 3. Helper would add some arguments and calls execve() again.
 * 4. Under layer redirects argv[0] to original path.
 * Pid would keep same.
 */
int main(int argc, char *argv[], char *envp[])
{
    // Try to get executable path
    const char *filename = get_current_exec();
    if (filename == NULL) {
        return -ENOENT;
    }

    // If there is no env, stop helper
    const char *helper_env = get_helper_env();
    if (helper_env == NULL) {
        return execve(filename, argv, envp);
    }

    // If there is no output, stop helper
    if (find_output_flag(argc, argv) < 0) {
        return execve(filename, argv, envp);
    }

    int new_argc = argc + APPEND_ARG_LEN + 1; // include terminator NULL
    char **new_argv = calloc(1, (unsigned long)new_argc * sizeof(char *));
    if (new_argv == NULL) {
        return execve(filename, argv, envp);
    }

    // Copy original arguments
    new_argc = 0;
    for (int i = 0; i < argc; i++) {
        if (argv[i] == NULL) {
            break;
        }
        new_argv[new_argc++] = argv[i];
    }

    // Write new arguments
    for (int i = 0; i < APPEND_ARG_LEN; i++) {
        new_argv[new_argc++] = APPEND_ARGS[i];
    }
    new_argv[new_argc] = NULL;

    return execve(filename, (char* const*)new_argv, envp);
}
