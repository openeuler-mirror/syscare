// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * gnu-as-hijacker is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

#define _GNU_SOURCE

#include <stdio.h>
#include <unistd.h>

#include <sys/stat.h>
#include <sys/syscall.h>

#include "hijacker.h"

#ifndef SYS_gettid
#error "SYS_gettid is unavailable on this system"
#endif

#define DEFSYM_MAX 64

static char *DEFSYM_FLAG = "--defsym";
static const int APPEND_ARG_LEN = 2;

static const char *NULL_DEV_PATH = "/dev/null";

static char g_defsym[DEFSYM_MAX] = { 0 };
static char g_new_output_file[PATH_MAX] = { 0 };

/*
 * The whole part:
 * 1. Someone called execve() to run a compiler (inode).
 * 2. If the inode was registered, under layer would rewrite argv[0] to hijacker path.
 * 3. Hijacker would add some arguments and calls execve() again.
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

    // If there is no env, stop hijack
    const char *output_dir = get_hijacker_env();
    if (output_dir == NULL) {
        return execve(filename, argv, envp);
    }

    // If output dir is not a directory, stop hijack
    struct stat output_dir_stat;
    if ((stat(output_dir, &output_dir_stat) != 0) ||
        (!S_ISDIR(output_dir_stat.st_mode))) {
        return execve(filename, argv, envp);
    }

    // If there is no output, stop hijack
    int output_index = find_output_flag(argc, argv);
    if (output_index < 0) {
        return execve(filename, argv, envp);
    }
    output_index += 1;

    // If the output is null device, stop hijack
    const char *output_file = argv[output_index];
    if (strncmp(output_file, NULL_DEV_PATH, strlen(NULL_DEV_PATH)) == 0) {
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
    pid_t tid = (pid_t)syscall(SYS_gettid);
    char *defsym_value = (char *)g_defsym;
    char *new_output_file = (char *)g_new_output_file;

    snprintf(defsym_value, DEFSYM_MAX, ".upatch_0x%x=", tid);
    new_argv[new_argc++] = DEFSYM_FLAG;
    new_argv[new_argc++] = defsym_value;
    new_argv[new_argc] = NULL;

    // Handle output file
    snprintf(new_output_file, PATH_MAX, "%s/0x%x.o", output_dir, tid);
    new_argv[output_index] = new_output_file;

    if (access(output_file, F_OK) == 0) {
        (void)unlink(output_file);
    }

    if (symlink(new_output_file, output_file) != 0) {
        return execve(filename, argv, envp);
    }

    return execve(filename, (char* const*)new_argv, envp);
}
