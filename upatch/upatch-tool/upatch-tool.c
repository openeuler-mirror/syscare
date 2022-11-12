#include <stdio.h>
#include <fcntl.h>

#include <linux/limits.h>
#include <sys/ioctl.h>

#include "upatch-ioctl.h"

int main()
{
    char path[PATH_MAX];
    int ret;
    int upatch_fd;

    snprintf(path, PATH_MAX, "/dev/%s", UPATCH_DEV_NAME);

    ret = open(path, O_RDWR);
    if (ret < 0) {
        printf("open failed - %d \n", ret);
        return ret;
    }
    upatch_fd = ret;

    ret = ioctl(upatch_fd, UPATCH_REGISTER_COMPILER, "/usr/bin/gcc");
    if (ret < 0) {
        printf("register the compiler - %d \n", ret);
        return ret;
    }

    ret = ioctl(upatch_fd, UPATCH_REGISTER_ASSEMBLER, "/usr/bin/as");
    if (ret < 0) {
        printf("register the assembler - %d \n", ret);
    }
    
    printf("everything works fine \n");

    return 0;
}