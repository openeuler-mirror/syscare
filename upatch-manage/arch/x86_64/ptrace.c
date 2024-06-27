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

#include <string.h>
#include <stdlib.h>

#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/syscall.h>

#include "upatch-ptrace.h"

long upatch_arch_syscall_remote(struct upatch_ptrace_ctx *pctx, int nr,
			       unsigned long arg1, unsigned long arg2,
			       unsigned long arg3, unsigned long arg4,
			       unsigned long arg5, unsigned long arg6,
			       unsigned long *res)
{
	struct user_regs_struct regs;

	unsigned char syscall[] = {
		0x0f, 0x05, /* syscall */
		0xcc, /* int3 */
	};
	long ret;

	memset(&regs, 0, sizeof(struct user_regs_struct));
	log_debug("Executing syscall %d (pid %d)...\n", nr, pctx->pid);
	regs.rax = (unsigned long long)nr;
	regs.rdi = arg1;
	regs.rsi = arg2;
	regs.rdx = arg3;
	regs.r10 = arg4;
	regs.r8 = arg5;
	regs.r9 = arg6;

	ret = upatch_execute_remote(pctx, syscall, sizeof(syscall), &regs);
	if (ret == 0)
		*res = regs.rax;

	return ret;
}

long upatch_arch_execute_remote_func(struct upatch_ptrace_ctx *pctx,
				    const unsigned char *code, size_t codelen,
				    struct user_regs_struct *pregs,
				    int (*func)(struct upatch_ptrace_ctx *pctx,
						const void *data),
				    const void *data)
{
	struct user_regs_struct orig_regs, regs;
	long ret;
	struct upatch_process *proc = pctx->proc;
	unsigned long libc_base = proc->libc_base;

	unsigned char *orig_code = (unsigned char *)malloc(sizeof(*orig_code) * codelen);

	if (orig_code == NULL) {
		log_error("Malloc orig_code failed\n");
		return -1;
	}
	ret = ptrace(PTRACE_GETREGS, pctx->pid, NULL, &orig_regs);
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
	regs.rip = libc_base;

	copy_regs(&regs, pregs);

	ret = ptrace(PTRACE_SETREGS, pctx->pid, NULL, &regs);
	if (ret < 0) {
		log_error("can't set regs - %d\n", pctx->pid);
		goto poke_back;
	}

	ret = func(pctx, data);
	if (ret < 0) {
		log_error("failed call to func\n");
		goto poke_back;
	}

	ret = ptrace(PTRACE_GETREGS, pctx->pid, NULL, &regs);
	if (ret < 0) {
		log_error("can't get updated regs - %d\n", pctx->pid);
		goto poke_back;
	}

	ret = ptrace(PTRACE_SETREGS, pctx->pid, NULL, &orig_regs);
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
	COPY_REG(r15);
	COPY_REG(r14);
	COPY_REG(r13);
	COPY_REG(r12);
	COPY_REG(rbp);
	COPY_REG(rbx);
	COPY_REG(r11);
	COPY_REG(r10);
	COPY_REG(r9);
	COPY_REG(r8);
	COPY_REG(rax);
	COPY_REG(rcx);
	COPY_REG(rdx);
	COPY_REG(rsi);
	COPY_REG(rdi);
#undef COPY_REG
}

#define UPATCH_INSN_LEN 6
#define UPATCH_ADDR_LEN 8
#define ORIGIN_INSN_LEN (UPATCH_INSN_LEN + UPATCH_ADDR_LEN)
size_t get_origin_insn_len()
{
	return ORIGIN_INSN_LEN;
}

size_t get_upatch_insn_len()
{
    return UPATCH_INSN_LEN;
}

size_t get_upatch_addr_len()
{
    return UPATCH_ADDR_LEN;
}


unsigned long get_new_insn(void)
{
	char jmp_insn[] = { 0xff, 0x25, 0x00, 0x00, 0x00, 0x00};
	return *(unsigned long *)jmp_insn;
}