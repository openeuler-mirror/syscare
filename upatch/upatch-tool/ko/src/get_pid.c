#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <string.h>
#include <limits.h>

#include "ioctl.h"

void __tool_exit(const char *str)
{
	perror(str);
	exit(EXIT_FAILURE);
}

elf_request_t* build_elf_request(char *path, loff_t offset, pid_t pid)
{
	char buf[PATH_MAX];

	char *elf_path = realpath(path, buf);

	elf_request_t *req = calloc(sizeof(elf_request_t), 1);
	if (!req) {
		return NULL;
	}

	memcpy(req->elf_path, elf_path, strlen(elf_path));
	req->offset = offset;
	req->monitor_pid = pid;

	return req;
}

pid_t get_pid(char *path, loff_t offset, pid_t monitor_pid)
{
	int ret = 0;
	pid_t pid;
	int upatch_fd = open("/dev/upatch-manager", O_RDWR);

	if (upatch_fd < 0) {
		__tool_exit("Error: Failed to open device /dev/upatch-manager");
	}

	ret = ioctl(upatch_fd, UPATCH_REGISTER_MONITOR, NULL);
	if (ret < 0) {
		free(req);
		__tool_exit("Error: ioctl failed");
	}

	elf_request_t *req = build_elf_request(argv);
	ret = ioctl(upatch_fd, UPATCH_REGISTER_ELF, req);
	if (ret < 0) {
		free(req);
		__tool_exit("Error: ioctl failed");
	}
	ret = ioctl(upatch_fd, UPATCH_GET_PID, &pid);
	if (ret < 0) {
		printf("get pid failed\n");
		return ret;
	}
	printf("upatch get pid %d, monitor pid %d\n", pid, monitor_pid);
	ret = ioctl(upatch_fd, UPATCH_DEREGISTER_ELF, req);
	if (ret < 0) {
		free(req);
		__tool_exit("Error: ioctl failed");
	}

	printf("upatch deregister monitor %d\n", monitor_pid);
	ret = ioctl(upatch_fd, UPATCH_DEREGISTER_MONITOR, &monitor_pid);
	if (ret < 0) {
		free(req);
		__tool_exit("Error: ioctl failed");
	}
	close(upatch_fd);
	return pid;
}
