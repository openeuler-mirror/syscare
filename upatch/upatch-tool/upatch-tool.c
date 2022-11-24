// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Zongwu Li <lizongwu@huawei.com>
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <error.h>
#include <argp.h>
#include <stdbool.h>
#include <fcntl.h>

#include <sys/ioctl.h>

#include "upatch-manage.h"
#include "upatch-ioctl.h"
#include "upatch-resolve.h"

#define COMMAND_SIZE 9
char* command[COMMAND_SIZE] =
    {"", "active", "deactive", "install", "uninstall", "apply", "remove", "info", "resolve"};
enum Command {
    DEFAULT,
    ACTIVE,
    DEACTIVE,
    INSTALL,
    UNINSTALL,
    APPLY,
    REMOVE,
    INFO,
    RESOLVE,
};

struct arguments {
    int cmd;
    struct upatch_conmsg upatch;
    bool debug;
};

static struct argp_option options[] = {
        {"cmd", 0, "command", 0, "active/deactive/install/uninstall/apply/remove/info/resolve"},
        {"binary", 'b', "binary", 0, "Binary file"},
        {"patch", 'p', "patch", 0, "Patch file"},
        {NULL}
};

static char program_doc[] = "upatch-tool -- apply a patch on binary";

static char args_doc[] = "cmd -b binary -p patch";

static error_t check_opt(struct argp_state *state)
{
    struct arguments *arguments = state->input;

    if (arguments->cmd == DEFAULT) {
        argp_usage(state);
        return ARGP_ERR_UNKNOWN;
    }
    switch (arguments->cmd) {
        case APPLY:
        case INSTALL:
        case RESOLVE:
            if (arguments->upatch.binary == NULL || arguments->upatch.patch == NULL) {
                argp_usage(state);
                return ARGP_ERR_UNKNOWN;
            }
        case ACTIVE:
        case DEACTIVE:
        case UNINSTALL:
        case REMOVE:
        case INFO:
            if (arguments->upatch.binary == NULL && arguments->upatch.patch == NULL) {
                argp_usage(state);
                return ARGP_ERR_UNKNOWN;
            }
        default:
            break;
    }
    return 0;
}

static error_t parse_opt(int key, char *arg, struct argp_state *state)
{
    struct arguments *arguments = state->input;

    switch (key)
    {
        case 'b':
            arguments->upatch.binary = arg;
            break;
        case 'p':
            arguments->upatch.patch = arg;
            break;
        case ARGP_KEY_ARG:
            if (state->arg_num >= 1)
                argp_usage (state);
            if (arguments->cmd != DEFAULT)
                argp_usage (state);
            for(int i = 1; i < COMMAND_SIZE; ++i) {
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

static struct argp argp = {options, parse_opt, args_doc, program_doc};

static int upatch_fd = -1;

void prepare_env(struct arguments *arguments)
{
    char path[PATH_MAX];

    if (arguments->cmd == RESOLVE)
        return;

    snprintf(path, PATH_MAX, "/dev/%s", UPATCH_DEV_NAME);
    upatch_fd = open(path, O_RDWR);
    if (upatch_fd < 0)
        error(errno, 0, "ERROR - %d: open failed %s", errno, path);

    return;
}

void active(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_ACTIVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: active", errno);
    }
}

void deactive(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_DEACTIVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: deactive", errno);
    }
}

void install(struct upatch_conmsg* upatch) {
    int ret = ioctl(upatch_fd, UPATCH_ATTACH_PATCH, upatch);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: install", errno);
    }
}

void uninstall(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_REMOVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: uninstall", errno);
    }
}

void info(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_INFO_PATCH, file);
    char *status;
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: info", errno);
    }
    switch (ret)
    {
    case UPATCH_STATE_ATTACHED:
        status = "installed";
        break;
    case UPATCH_STATE_RESOLVED:
        status = "deactived";
        break;
    case UPATCH_STATE_ACTIVED:
        status = "actived";
        break;
    case UPATCH_STATE_REMOVED:
        status = "removed";
        break;
    default:
        break;
    }
    printf("Status: %s \n", status);
}

int main(int argc, char*argv[])
{
    int ret = 0;
    struct arguments arguments;
    const char* file;

    memset(&arguments, 0, sizeof(arguments));
    argp_parse(&argp, argc, argv, 0, NULL, &arguments);

    prepare_env(&arguments);

    file = arguments.upatch.binary;
    if (file == NULL)
        file = arguments.upatch.patch;

    switch (arguments.cmd) {
        case ACTIVE:
            active(file);
            break;
        case DEACTIVE:
            deactive(file);
            break;
        case INSTALL:
            install(&arguments.upatch);
            break;
        case UNINSTALL:
            uninstall(file);
            break;
        case APPLY:
            install(&arguments.upatch);
            active(file);
            break;
        case REMOVE:
            deactive(file);
            uninstall(file);
            break;
        case INFO:
            info(file);
            break;
        case RESOLVE:
            ret = resolve_patch(arguments.upatch.binary, arguments.upatch.patch);
            if (ret)
                printf("resolv patch failed - %d \n", ret);
            break;
        default:
            error(-1, 0, "ERROR - -1: no this cmd");
            break;
    }

    return ret;
}