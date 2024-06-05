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

#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <stdlib.h>

#include "insn.h"
#include "upatch-ptrace.h"

#define ORIGIN_INSN_LEN 16

long upatch_arch_syscall_remote(struct upatch_ptrace_ctx *pctx, int nr,
			       unsigned long arg1, unsigned long arg2,
			       unsigned long arg3, unsigned long arg4,
			       unsigned long arg5, unsigned long arg6,
			       unsigned long *res)
{
	struct user_regs_struct regs;
	unsigned char syscall[] = {
		0x01, 0x00, 0x00, 0xd4, // 0xd4000001 svc #0  = syscall
		0xa0, 0x00, 0x20, 0xd4, // 0xd42000a0 brk #5  = int3
	};
	long ret;

	log_debug("Executing syscall %d (pid %d)...\n", nr, pctx->pid);
	regs.regs[8] = (unsigned long long)nr;
	regs.regs[0] = arg1;
	regs.regs[1] = arg2;
	regs.regs[2] = arg3;
	regs.regs[3] = arg4;
	regs.regs[4] = arg5;
	regs.regs[5] = arg6;

	ret = upatch_execute_remote(pctx, syscall, sizeof(syscall), &regs);
	if (ret == 0)
		*res = regs.regs[0];

	return ret;
}

long upatch_arch_execute_remote_func(struct upatch_ptrace_ctx *pctx,
				    const unsigned char *code, size_t codelen,
				    struct user_regs_struct *pregs,
				    int (*func)(struct upatch_ptrace_ctx *pctx,
						const void *data),
				    const void *data)
{
	long ret;
	struct user_regs_struct orig_regs, regs;
	struct iovec orig_regs_iov, regs_iov;
	struct upatch_process *proc = pctx->proc;
	unsigned long libc_base = proc->libc_base;
	unsigned char *orig_code = (unsigned char *)malloc(sizeof(*orig_code) * codelen);

	if (orig_code == NULL) {
		log_error("Malloc orig_code failed\n");
		return -1;
	}

	orig_regs_iov.iov_base = &orig_regs;
	orig_regs_iov.iov_len = sizeof(orig_regs);
	regs_iov.iov_base = &regs;
	regs_iov.iov_len = sizeof(regs);

	ret = ptrace(PTRACE_GETREGSET, pctx->pid, (void *)NT_PRSTATUS,
		     (void *)&orig_regs_iov);
	if (ret < 0) {
		log_error("can't get regs - %d\n", pctx->pid);
		free(orig_code);
		return -1;
	}
	ret = upatch_process_mem_read(proc, libc_base,
				      (unsigned long *)orig_code, codelen);
	if (ret < 0) {
		log_error("can't peek original code - %d\n", pctx->pid);
		free(orig_code);
		return -1;
	}
	ret = upatch_process_mem_write(proc, code, libc_base,
				       codelen);
	if (ret < 0) {
		log_error("can't poke syscall code - %d\n", pctx->pid);
		goto poke_back;
	}

	regs = orig_regs;
	regs.pc = libc_base;

	copy_regs(&regs, pregs);

	ret = ptrace(PTRACE_SETREGSET, pctx->pid, (void *)NT_PRSTATUS,
		     (void *)&regs_iov);
	if (ret < 0) {
		log_error("can't set regs - %d\n", pctx->pid);
		goto poke_back;
	}

	ret = func(pctx, data);
	if (ret < 0) {
		log_error("failed call to func\n");
		goto poke_back;
	}

	ret = ptrace(PTRACE_GETREGSET, pctx->pid, (void *)NT_PRSTATUS,
		     (void *)&regs_iov);
	if (ret < 0) {
		log_error("can't get updated regs - %d\n", pctx->pid);
		goto poke_back;
	}

	ret = ptrace(PTRACE_SETREGSET, pctx->pid, (void *)NT_PRSTATUS,
		     (void *)&orig_regs_iov);
	if (ret < 0) {
		log_error("can't restore regs - %d\n", pctx->pid);
		goto poke_back;
	}

	*pregs = regs;

poke_back:
	upatch_process_mem_write(proc, (unsigned long *)orig_code, libc_base,
				 codelen);
	free(orig_code);
	return ret;
}

void copy_regs(struct user_regs_struct *dst, struct user_regs_struct *src)
{
#define COPY_REG(x) dst->x = src->x
	COPY_REG(regs[0]);
	COPY_REG(regs[1]);
	COPY_REG(regs[2]);
	COPY_REG(regs[3]);
	COPY_REG(regs[4]);
	COPY_REG(regs[5]);
	COPY_REG(regs[8]);
	COPY_REG(regs[29]);

	COPY_REG(regs[9]);
	COPY_REG(regs[10]);
	COPY_REG(regs[11]);
	COPY_REG(regs[12]);
	COPY_REG(regs[13]);
	COPY_REG(regs[14]);
	COPY_REG(regs[15]);
	COPY_REG(regs[16]);
	COPY_REG(regs[17]);
	COPY_REG(regs[18]);
	COPY_REG(regs[19]);
	COPY_REG(regs[20]);
#undef COPY_REG
}

size_t get_origin_insn_len(void)
{
	return ORIGIN_INSN_LEN;
}
#define UPATCH_INSN_LEN 8
#define UPATCH_ADDR_LEN 8
size_t get_upatch_insn_len(void)
{
	return UPATCH_INSN_LEN;
}

size_t get_upatch_addr_len(void)
{
	return UPATCH_ADDR_LEN;
}

// for long jumper
unsigned long get_new_insn(void)
{
	unsigned int insn0 = 0x58000051; // ldr x17, #8
	unsigned int insn4 = 0xd61f0220; // br x17
	return (unsigned long)(insn0 | ((unsigned long)insn4 << 32));
}