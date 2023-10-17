// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
/* Copyright (c) 2023 Longjun Luo. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

#include <bpf/libbpf.h>
#include <sys/resource.h>
#include <sys/stat.h>

#include "upatch-manager.h"
#include "upatch-manager.skel.h"

static int libbpf_print_fn(enum libbpf_print_level level, const char *format, va_list args)
{
	return vfprintf(stderr, format, args);
}

static int attach_elf(struct upatch_manager_bpf *skel, const char *path)
{
	struct bpf_link *link;
	int ret;

	/* find entry point and attach it */
	link = bpf_program__attach_uprobe(skel->progs.upatch_empty_handler, false, -1, path, 0x0);
	if (!link) {
		ret = -errno;
		fprintf(stderr, "Failed to attach 1 uprobe: %d\n", ret);
		return ret;
	}
	return 0;
}

/* TODO: find all pids and handle them */
static int upatch_manage_daemon()
{
	sleep(1);
	return 0;
}

int main(int argc, char **argv)
{
	int err;
	struct upatch_manager_bpf *skel;

	libbpf_set_print(libbpf_print_fn);

	skel = upatch_manager_bpf__open_and_load();
	if (!skel) {
		fprintf(stderr, "Failed to open and load BPF skeleton\n");
		return 1;
	}

	skel->links.install_breakpoint = bpf_program__attach_kprobe(
		skel->progs.install_breakpoint, false, "install_breakpoint.isra.0");
	if (!skel->links.install_breakpoint && errno == ENOENT)
		skel->links.install_breakpoint = bpf_program__attach_kprobe(
			skel->progs.install_breakpoint, false, "install_breakpoint");
	if (!skel->links.install_breakpoint) {
		err = -errno;
		fprintf(stderr, "Failed to attach kprobe for install_breakpoint: %d \n", err);
		goto cleanup;
	}

	err = upatch_manager_bpf__attach(skel);
	if (err) {
		fprintf(stderr, "Failed to auto-attach BPF skeleton: %d\n", err);
		goto cleanup;
	}

	while(1) {
		upatch_manage_daemon();
	}

cleanup:
	upatch_manager_bpf__destroy(skel);
	return -err;
}
