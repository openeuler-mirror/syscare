#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <error.h>
#include <argp.h>
#include <stdbool.h>
#include <fcntl.h>

#include <sys/ioctl.h>

#include "upatch-ioctl.h"

#define COMMAND_SIZE 7
char* command[COMMAND_SIZE] = {"", "active", "deactive", "install", "uninstall", "apply", "remove"};
enum Command {
    DEFAULT,
    ACTIVE,
    DEACTIVE,
    INSTALL,
    UNINSTALL,
    APPLY,
    REMOVE,
};

struct arguments {
    int cmd;
    struct upatch_conmsg upatch;
    bool debug;
};

static struct argp_option options[] = {
        {"cmd", 0, "command", 0, "active/deactive/install/uninstall/apply/remove"},
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
            if (arguments->upatch.binary == NULL || arguments->upatch.patch == NULL) {
                argp_usage(state);
                return ARGP_ERR_UNKNOWN;
            }
        case ACTIVE:
        case DEACTIVE:
        case UNINSTALL:
        case REMOVE:
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

/* Format of output file is the only export API */
static void show_program_info(struct arguments *arguments)
{
    printf("cmd: %d\n", arguments->cmd);
    printf("binary file: %s\n", arguments->upatch.binary);
    printf("patch file: %s\n", arguments->upatch.patch);
}

void active(int upatch_fd, const char *file){
    int ret = ioctl(upatch_fd, UPATCH_ACTIVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: active", errno);
    }
}

void deactive(int upatch_fd, const char *file){
    int ret = ioctl(upatch_fd, UPATCH_DEACTIVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: deactive", errno);
    }
}

void install(int upatch_fd, struct upatch_conmsg* upatch){
    int ret = ioctl(upatch_fd, UPATCH_ATTACH_PATCH, upatch);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: install", errno);
    }
}

void uninstall(int upatch_fd, const char *file){
    int ret = ioctl(upatch_fd, UPATCH_REMOVE_PATCH, file);
    if (ret < 0) {
        error(errno, 0, "ERROR - %d: uninstall", errno);
    }
}

int main(int argc, char*argv[])
{
    struct arguments arguments;
    char path[PATH_MAX];
    int upatch_fd;

    memset(&arguments, 0, sizeof(arguments));
    argp_parse(&argp, argc, argv, 0, NULL, &arguments);

    snprintf(path, PATH_MAX, "/dev/%s", UPATCH_DEV_NAME);
    upatch_fd = open(path, O_RDWR);
    if (upatch_fd < 0){
        error(errno, 0, "ERROR - %d: open failed %s", errno, path);
    }

    const char* file = (arguments.upatch.binary == NULL) ? arguments.upatch.patch : arguments.upatch.binary;

    switch (arguments.cmd) {
        case ACTIVE:
            active(upatch_fd, file);
            break;
        case DEACTIVE:
            deactive(upatch_fd, file);
            break;
        case INSTALL:
            install(upatch_fd, &arguments.upatch);
            break;
        case UNINSTALL:
            uninstall(upatch_fd, file);
            break;
        case APPLY:
            install(upatch_fd, &arguments.upatch);
            active(upatch_fd, file);
            break;
        case REMOVE:
            deactive(upatch_fd, file);
            uninstall(upatch_fd, file);
            break;
        default:
            error(-1, 0, "ERROR - -1: no this cmd");
            break;
    }

    return 0;
}