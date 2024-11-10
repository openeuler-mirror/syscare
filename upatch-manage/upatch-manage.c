// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
 * Copyright (C) 2024 Huawei Technologies Co., Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */

#include <argp.h>
#include <dirent.h>
#include <libgen.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "log.h"
#include "upatch-elf.h"
#include "upatch-patch.h"
#include "upatch-stack-check.h"

#define PROG_VERSION "upatch-manage " BUILD_VERSION
#define COMMAND_SIZE 4

enum loglevel loglevel = NORMAL;
char* logprefix;

char* command[COMMAND_SIZE] = {"", "patch", "unpatch", "info"};
enum Command {
    DEFAULT,
    PATCH,
    UNPATCH,
    INFO,
};

struct arguments {
    int cmd;
    int pid;
    char* upatch;
    char* binary;
    char* uuid;
    bool verbose;
};

static struct argp_option options[] = {
    {"verbose", 'v', NULL, 0, "Show verbose output", 0},
    {"uuid", 'U', "uuid", 0, "the uuid of the upatch", 0},
    {"pid", 'p', "pid", 0, "the pid of the user-space process", 0},
    {"upatch", 'u', "upatch", 0, "the upatch file", 0},
    {"binary", 'b', "binary", 0, "the binary file", 0},
    {"cmd", 0, "patch", 0, "Apply a upatch file to a user-space process", 0},
    {"cmd", 0, "unpatch", 0, "Unapply a upatch file to a user-space process", 0},
    {NULL}};

static char program_doc[] = "Operate a upatch file on the user-space process";

static char args_doc[] =
    "<cmd> --pid <Pid> --upatch <Upatch path> --binary <Binary path> --uuid "
    "<Uuid>";

const char* argp_program_version = PROG_VERSION;

static error_t check_opt(struct argp_state* state)
{
    struct arguments* arguments = state->input;

    if (arguments->cmd == DEFAULT) {
        argp_usage(state);
        return ARGP_ERR_UNKNOWN;
    }
    switch (arguments->cmd) {
        case PATCH:
        case UNPATCH:
        case INFO:
            if (!arguments->pid || arguments->upatch == NULL ||
                arguments->binary == NULL || arguments->uuid == NULL) {
                argp_usage(state);
                return ARGP_ERR_UNKNOWN;
            }
        default:
            break;
    }
    return 0;
}

static error_t parse_opt(int key, char* arg, struct argp_state* state)
{
    struct arguments* arguments = state->input;

    switch (key) {
        case 'v':
            arguments->verbose = true;
            break;
        case 'p':
            arguments->pid = atoi(arg);
            break;
        case 'u':
            arguments->upatch = arg;
            break;
        case 'b':
            arguments->binary = arg;
            break;
        case 'U':
            arguments->uuid = arg;
            break;
        case ARGP_KEY_ARG:
            if (state->arg_num >= 1) {
                argp_usage(state);
            }
            if (arguments->cmd != DEFAULT) {
                argp_usage(state);
            }
            for (int i = 1; i < COMMAND_SIZE; ++i) {
                if (!strcmp(arg, command[i])) {
                    arguments->cmd = i;
                    break;
                }
            }
            break;
        case ARGP_KEY_END:
            return check_opt(state);
        default:
            return ARGP_ERR_UNKNOWN;
    }
    return 0;
}

static struct argp argp = {options, parse_opt, args_doc, program_doc,
                           NULL,    NULL,      NULL};

int patch_upatch(const char* uuid, const char* binary_path,
                 const char* upatch_path, int pid)
{
    struct upatch_elf uelf;
    struct running_elf relf;
    memset(&uelf, 0, sizeof(struct upatch_elf));
    memset(&relf, 0, sizeof(struct running_elf));

    int ret = upatch_init(&uelf, upatch_path);
    if (ret) {
        log_error("Failed to initialize patch, pid=%d, ret=%d\n", pid, ret);
        goto out;
    }

    ret = process_patch(pid, &uelf, &relf, uuid, binary_path);
    if (ret) {
        log_error("Failed to patch process, pid=%d, ret=%d\n", pid, ret);
        goto out;
    }

out:
    upatch_close(&uelf);
    binary_close(&relf);

    return ret;
}

int unpatch_upatch(const char* uuid, int pid)
{
    int ret = 0;

    ret = process_unpatch(pid, uuid);
    if (ret) {
        log_error("Failed to unpatch process, pid=%d, ret=%d\n", pid, ret);
        return ret;
    }

    return 0;
}

int info_upatch(int pid)
{
    int ret = process_info(pid);
    if (ret != 0) {
        log_error("Failed to get patch info, pid=%d, ret=%d\n", pid, ret);
        return ret;
    }

    return 0;
}

int main(int argc, char* argv[])
{
    struct arguments args;
    int ret;

    memset(&args, 0, sizeof(struct arguments));
    argp_parse(&argp, argc, argv, 0, NULL, &args);
    if (args.verbose) {
        loglevel = DEBUG;
    }

    logprefix = basename(args.upatch);
    log_debug("PID: %d\n", args.pid);
    log_debug("UUID: %s\n", args.uuid);
    log_debug("Patch: %s\n", args.upatch);
    log_debug("Binary: %s\n", args.binary);

    args.pid = args.pid & INT32_MAX;
    switch (args.cmd) {
        case PATCH:
            ret = patch_upatch(args.uuid, args.binary, args.upatch, args.pid);
            break;
        case UNPATCH:
            ret = unpatch_upatch(args.uuid, args.pid);
            break;
        case INFO:
            ret = info_upatch(args.pid);
            break;
        default:
            ERROR("Unknown command");
            ret = EINVAL;
            break;
    }

    return abs(ret);
}
