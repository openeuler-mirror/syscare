// SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
/* Copyright (c) 2023 Longjun Luo. */
#include "vmlinux.h"

#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

#include "upatch-manager.h"

unsigned int _version SEC("version") = 1;
char LICENSE[] SEC("license") = "Dual BSD/GPL";

struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, UPATCH_MAX_PATCH_ENTITY);
	__type(key, struct elf_process);
	__type(value, int);
} elf_process_maps SEC(".maps");

static int initial_entry = 0;

SEC("uprobe")
/* used for find releated process, need check it when using pid */
int upatch_empty_handler(struct pt_regs *ctx)
{
	return 0;
}

SEC("kprobe")
/* ATTENTION: install_breakpoint is a local function and it may repeat */
int BPF_KPROBE(install_breakpoint, struct uprobe *uprobe, struct mm_struct *mm,
	struct vm_area_struct *vma, unsigned long vaddr)
{
	struct elf_process ep;
	ep.ino = BPF_CORE_READ(uprobe, inode, i_ino);
	ep.pid = BPF_CORE_READ(mm, owner, pid);
	bpf_map_update_elem(&elf_process_maps, &ep, &initial_entry, BPF_ANY);
	bpf_printk("ino %lu works for pid %d in addr 0x%lx \n", ep.ino, ep.pid, vaddr);
	return 0;
}