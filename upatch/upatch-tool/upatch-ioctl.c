#include "upatch-ioctl.h"

#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <string.h>
#include <sys/types.h>
#include <sys/ioctl.h>

#include "list.h"
#include "log.h"

static const char *UPATCH_DEV = "/dev/upatch_manager";

elf_request_t* build_elf_request(const char *elf_path, const char *patch_path, loff_t offset, pid_t monitor_pid)
{
	char e_buf[PATH_MAX];
	char p_buf[PATH_MAX];

	char *e_path = realpath(elf_path, e_buf);
	char *p_path = realpath(patch_path, p_buf);

	elf_request_t *req = calloc(sizeof(elf_request_t), 1);
	if (!req) {
		return NULL;
	}

	memcpy(req->elf_path, e_path, strlen(e_path));
	memcpy(req->patch_path, p_path, strlen(p_path));
	req->offset = offset;
	req->monitor_pid = monitor_pid;

	return req;
}

int patch_ioctl_apply(const char *target_path, const char *patch_path,
		struct list_head *symbol_list)
{
	// TODO: Call ioctl to request kernel driver to load patch
	// ioctl -> ko -> register uprobe -> uprobe handler -> execute upatch-manage
	pid_t monitor_pid = 0;
	elf_request_t *req = NULL;
	int ret = -1;
	patch_symbols_t *sym;
	int upatch_fd = open(UPATCH_DEV, O_RDWR);

	if (upatch_fd < 0) {
		log_warn("upatch-ioctl: open dev failed\n");
		goto out;
	}

	ret = ioctl(upatch_fd, UPATCH_REGISTER_MONITOR, NULL);
	if (ret < 0) {
		log_warn("upatch-ioctl: register monitor ioctl failed\n");
		goto err;
	}

	list_for_each_entry(sym, symbol_list, self) {
		// register_elf
		req = build_elf_request(target_path, patch_path, sym->offset, monitor_pid);
		if (!req) {
			log_warn("upatch-ioctl:build request failed\n");
			goto err;
		}

		ret = ioctl(upatch_fd, UPATCH_REGISTER_ELF, req);
		if (ret < 0) {
			free(req);
			log_warn("upatch-ioctl: register elf ioctl failed\n");
			goto err;
		}
		free(req);
		req = NULL;
	}
err:
	close(upatch_fd);
out:
	return ret;
}

int patch_ioctl_remove(const char *target_path, const char *patch_path,
		struct list_head *symbol_list)
{
	// TODO: Call ioctl to request kernel driver to remove patch
	// ioctl -> ko -> remove uprobe -> execute upatch-manage
	pid_t monitor_pid = 0;
	elf_request_t *req = NULL;
	int ret = -1;
	patch_symbols_t *sym;
	int upatch_fd = open(UPATCH_DEV, O_RDWR);

	if (upatch_fd < 0) {
		log_warn("upatch-ioctl: open dev failed\n");
		goto out;
	}

	list_for_each_entry(sym, symbol_list, self) {
		// register_elf
		req = build_elf_request(target_path, patch_path, sym->offset, monitor_pid);
		if (!req) {
			log_warn("upatch-ioctl:build request failed\n");
			goto err;
		}

		ret = ioctl(upatch_fd, UPATCH_DEREGISTER_ELF, req);
		if (ret < 0) {
			free(req);
			log_warn("upatch-ioctl: deregister elf ioctl failed\n");
			goto err;
		}
		monitor_pid = req->monitor_pid;
		free(req);
		req = NULL;
	}

	req = build_elf_request(target_path, patch_path, 0, monitor_pid);
	if (!req) {
		log_warn("upatch-ioctl:build request failed\n");
		goto err;
	}
	ret = ioctl(upatch_fd, UPATCH_REMOVE_PATCH, req);
	if (ret < 0) {
		free(req);
		log_warn("upatch-ioctl: remove patch ioctl failed\n");
		goto err;
	}
	free(req);
	req = NULL;

	monitor_pid = getpid();
	ret = ioctl(upatch_fd, UPATCH_DEREGISTER_MONITOR, &monitor_pid);
	if (ret < 0) {
		log_warn("upatch-ioctl: register monitor ioctl failed\n");
		goto err;
	}

err:
	close(upatch_fd);
out:
	return ret;
	return 0;
}
