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

#ifndef __UPATCH_PTRACE__
#define __UPATCH_PTRACE__

#include <sys/user.h>

#include "upatch-process.h"
#include "list.h"
#include "log.h"

#define MAX_ERRNO 4095

struct upatch_ptrace_ctx {
	int pid;
	int running;
	unsigned long execute_until;
	struct upatch_process *proc;
	struct list_head list;
};

#define proc2pctx(proc) \
	list_first_entry(&(proc)->ptrace.pctxs, struct upatch_ptrace_ctx, list)

int upatch_process_mem_read(struct upatch_process *proc, unsigned long src,
			    void *dst, size_t size);

int upatch_process_mem_write(struct upatch_process *, void *, unsigned long,
			     size_t);

int upatch_ptrace_attach_thread(struct upatch_process *, int);

int upatch_ptrace_detach(struct upatch_ptrace_ctx *);

int wait_for_stop(struct upatch_ptrace_ctx *, const void *);

void copy_regs(struct user_regs_struct *, struct user_regs_struct *);

int upatch_arch_execute_remote_func(struct upatch_ptrace_ctx *pctx,
				    const unsigned char *code, size_t codelen,
				    struct user_regs_struct *pregs,
				    int (*func)(struct upatch_ptrace_ctx *pctx,
						const void *data),
				    const void *data);

int upatch_arch_syscall_remote(struct upatch_ptrace_ctx *, int, unsigned long,
			       unsigned long, unsigned long, unsigned long,
			       unsigned long, unsigned long, unsigned long *);

unsigned long upatch_mmap_remote(struct upatch_ptrace_ctx *, unsigned long,
				 size_t, int, int, int, off_t);

int upatch_mprotect_remote(struct upatch_ptrace_ctx *, unsigned long, size_t,
			   int);

int upatch_munmap_remote(struct upatch_ptrace_ctx *, unsigned long, size_t);

int upatch_execute_remote(struct upatch_ptrace_ctx *, const unsigned char *,
			  size_t, struct user_regs_struct *);

size_t get_origin_insn_len();
size_t get_upatch_insn_len();
size_t get_upatch_addr_len();
unsigned long get_new_insn(struct object_file *, unsigned long, unsigned long);

#endif
