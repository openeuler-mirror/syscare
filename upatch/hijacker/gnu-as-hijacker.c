#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <limits.h>
#include <errno.h>
#include <string.h>

#include <sys/syscall.h>
#include <sys/stat.h>

#include "hijacker.h"

#ifndef SYS_gettid
#error "SYS_gettid unavailable on this system"
#endif

#define gettid() ((pid_t)syscall(SYS_gettid))

/* %u used to find object file and 0x0 use to match it */
#define DEFSYM_FORMAT     "upatch_tag_0x%x=0x0"

/*
 * ATTENTION: written by ebpf directly.
 * 
 * The whole part:
 * 1. the compiler path + other inode number(before execve) -> the hijacker
 * 2. the hijacker inode number + the hijacker path -> the compiler path(after execve)
 * 
 * Pid keeps the same.
 */
static char original_path[PATH_MAX] = {0xff};

int main(int argc, char *argv[], char *envp[])
{
    char buff[64];
    const char **__argv = (const char **)argv;
    const char *upatch_env = NULL, *object_path = NULL;
    int new_index = 1, old_index = 1;
    int tid = gettid();

    upatch_env = getenv(HIJACKER_ENV);
    if (!upatch_env)
        goto out;

    /* append NULL at the end of argv */
    __argv = calloc(sizeof(char *), argc + 2 + 1);
    if (!__argv)
        return -ENOMEM;
    
    /* add defsymbols */
    snprintf(buff, 64, DEFSYM_FORMAT, tid);
    __argv[new_index ++] = "--defsym";
    __argv[new_index ++] = buff;
    while (old_index < argc)
        __argv[new_index ++] = argv[old_index ++];
    __argv[new_index] = NULL;

    for (old_index = 1; old_index < new_index - 1; old_index ++) {
        if (__argv[old_index] && !strncmp(__argv[old_index], "-o", 2))
            break;
    }

    /* ATTENTION: consider @FILE. */
    object_path = __argv[old_index + 1]; // if not found, old_index + 1 = new_index
    if (!object_path)
        return -EINVAL;
    /* no handle for the case like: as -v --64 -o /dev/null /dev/null */
    else if (!strcmp(object_path, "/dev/null"))
        goto out;

    snprintf((char *)original_path, PATH_MAX, LINK_PATH_FORMAT, upatch_env, tid);
    /* check if the link path is the only one */
    if (!access(original_path, F_OK))
        return -EEXIST;
    __argv[old_index + 1] = strdup(original_path);

    unlink(object_path);
    if (symlink(original_path, object_path) == -1)
        return -errno;
out:
    /* fill the filename for the argument */
    __argv[0] = (char *)&original_path;
    return execve((const char *)&original_path, (void *)__argv, envp);
}