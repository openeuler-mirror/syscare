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
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <error.h>
#include <argp.h>
#include <stdbool.h>
#include <unistd.h>
#include <fcntl.h>

#include <sys/ioctl.h>

#include "upatch-manage.h"
#include "upatch-ioctl.h"

#define COMMAND_SIZE 8
char* command[COMMAND_SIZE] =
    {"", "active", "deactive", "install", "uninstall", "apply", "remove", "info"};
enum Command {
    DEFAULT,
    ACTIVE,
    DEACTIVE,
    INSTALL,
    UNINSTALL,
    APPLY,
    REMOVE,
    INFO,
};

struct arguments {
    int cmd;
    struct upatch_conmsg upatch;
    bool debug;
};

static struct argp_option options[] = {
        {"binary", 'b', "binary", 0, "Binary file"},
        {"patch", 'p', "patch", 0, "Patch file"},
        {"cmd", 0, "active", 0, "Active the patch (require binary or patch)"},
        {"cmd", 0, "deactive", 0, "Deactive the patch (require binary or patch)"},
        {"cmd", 0, "install", 0, "Install the patch"},
        {"cmd", 0, "uninstall", 0, "Uninstall the patch (require binary or patch)"},
        {"cmd", 0, "apply", 0, "Equivalent to: install and active"},
        {"cmd", 0, "remove", 0, "Equivalent to: deactive and uninstall"},
        {"cmd", 0, "info", 0, "Show status of the patch (require binary or patch)"},
        {NULL}
};

static char program_doc[] = "Operate a patch on binary";

static char args_doc[] = "<cmd> --binary <Binary file> --patch <Patch file>";

const char *argp_program_version = UPATCH_VERSION;

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

void __tool_exit(const char *str)
{
    perror(str);
    exit(EXIT_FAILURE);
}

void prepare_env(struct arguments *arguments)
{
    char path[PATH_MAX];

    snprintf(path, PATH_MAX, "/dev/%s", UPATCH_DEV_NAME);
    upatch_fd = open(path, O_RDWR);
    if (upatch_fd < 0)
        __tool_exit("open device failed");
}

void __check_files(const char *path)
{
    int fd = open(path, O_RDONLY);
    if (fd != -1)
        close(fd);
    else
        __tool_exit("open file failed");
}

void check_files(struct arguments *arguments)
{
    if (arguments->upatch.binary)
        __check_files(arguments->upatch.binary);
    if (arguments->upatch.patch)
        __check_files(arguments->upatch.patch);
}

void active(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_ACTIVE_PATCH, file);
    if (ret < 0)
        __tool_exit("active action failed");
}

void deactive(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_DEACTIVE_PATCH, file);
    if (ret < 0)
        __tool_exit("deactive action failed");
}

void install(struct upatch_conmsg* upatch) {
    int ret = ioctl(upatch_fd, UPATCH_ATTACH_PATCH, upatch);
    if (ret < 0)
        __tool_exit("install action failed");
}

void uninstall(const char *file) {
    int ret = ioctl(upatch_fd, UPATCH_REMOVE_PATCH, file);
    if (ret < 0)
        __tool_exit("uninstall action failed");
}

void info(const char *file) {
    char *status = "error";
    int ret = ioctl(upatch_fd, UPATCH_INFO_PATCH, file);
    if (errno == ENOENT)
        ret = UPATCH_STATE_REMOVED;

    if (ret < 0)
        __tool_exit("info action failed");

    switch (ret)
    {
    case UPATCH_STATE_REMOVED:
        status = "removed";
        break;
    case UPATCH_STATE_ATTACHED:
        status = "installed";
        break;
    case UPATCH_STATE_RESOLVED:
        status = "deactived";
        break;
    case UPATCH_STATE_ACTIVED:
        status = "actived";
        break;
    default:
        break;
    }
    printf("%s\n", status);
}

int main(int argc, char*argv[])
{
    int ret = 0;
    struct arguments arguments;
    const char* file;

    memset(&arguments, 0, sizeof(arguments));
    argp_parse(&argp, argc, argv, 0, NULL, &arguments);

    prepare_env(&arguments);
    check_files(&arguments);

    if (arguments.upatch.binary != NULL)
        file = arguments.upatch.binary;
    else
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
        default:
            fprintf(stderr, "unsupported command\n");
            exit(EXIT_FAILURE);
    }

    return ret;
}