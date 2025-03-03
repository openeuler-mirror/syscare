// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
 * Copyright (C) 2024 ISCAS
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
#include <asm/ptrace.h>

#include "upatch-ptrace.h"

int upatch_arch_reg_init(int pid, unsigned long *sp, unsigned long *pc)
{
    struct iovec regs_iov;
    struct user_regs_struct regs;

    regs_iov.iov_base = &regs;
    regs_iov.iov_len = sizeof(regs);

    if (ptrace(PTRACE_GETREGSET, pid,
               (void *)NT_PRSTATUS, (void *)&regs_iov) < 0) {
        log_error("Cannot get regs from %d\n", pid);
        return -1;
    }
    *sp = (unsigned long)regs.sp;
    *pc = (unsigned long)regs.pc;
    return 0;
}

static long read_gregs(int pid, struct user_regs_struct *regs)
{
    struct iovec data = {regs, sizeof(*regs)};
    if (ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &data) == -1) {
        log_error("ptrace(PTRACE_GETREGSET)");
        return -1;
    }
    return 0;
}

static long write_gregs(int pid, struct user_regs_struct *regs)
{
    struct iovec data = {regs, sizeof(*regs)};
    if (ptrace(PTRACE_SETREGSET, pid, NT_PRSTATUS, &data) == -1) {
        log_error("ptrace(PTRACE_SETREGSET)");
        return -1;
    }
    return 0;
}

long upatch_arch_syscall_remote(struct upatch_ptrace_ctx *pctx, int nr,
                                unsigned long arg1, unsigned long arg2,
                                unsigned long arg3, unsigned long arg4,
                                unsigned long arg5, unsigned long arg6,
                                unsigned long *res)
{
    struct user_regs_struct regs;
    unsigned char syscall[] = {
        0x73, 0x00, 0x00, 0x00, // ecall
        0x73, 0x00, 0x10, 0x00, // ebreak
    };
    long ret;

    log_debug("Executing syscall %d (pid %d)...\n", nr, pctx->pid);
    regs.a7 = (unsigned long)nr;
    regs.a0 = arg1;
    regs.a1 = arg2;
    regs.a2 = arg3;
    regs.a3 = arg4;
    regs.a4 = arg5;
    regs.a5 = arg6;

    ret = upatch_execute_remote(pctx, syscall, sizeof(syscall), &regs);
    if (ret == 0)
        *res = regs.a0;

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
    unsigned char orig_code[codelen];
    long ret;
    struct upatch_process *proc = pctx->proc;
    unsigned long libc_base = proc->libc_base;

    ret = read_gregs(pctx->pid, &orig_regs);
    if (ret < 0) {
        return -1;
    }
    ret = upatch_process_mem_read(proc, libc_base,
                                  (unsigned long *)orig_code, codelen);
    if (ret < 0) {
        log_error("can't peek original code - %d\n", pctx->pid);
        return -1;
    }
    ret = upatch_process_mem_write(proc, (const unsigned long *)code, libc_base,
                                   codelen);
    if (ret < 0) {
        log_error("can't poke syscall code - %d\n", pctx->pid);
        goto poke_back;
    }

    regs = orig_regs;
    regs.pc = libc_base;

    copy_regs(&regs, pregs);

    ret = write_gregs(pctx->pid, &regs);
    if (ret < 0) {
        goto poke_back;
    }

    ret = func(pctx, data);
    if (ret < 0) {
        log_error("failed call to func\n");
        goto poke_back;
    }

    ret = read_gregs(pctx->pid, &regs);
    if (ret < 0) {
        goto poke_back;
    }

    ret = write_gregs(pctx->pid, &orig_regs);
    if (ret < 0) {
        goto poke_back;
    }
    
    *pregs = regs;

poke_back:
    upatch_process_mem_write(proc, (unsigned long *)orig_code, libc_base,
                             codelen);
    return ret;
}

void copy_regs(struct user_regs_struct *dst, struct user_regs_struct *src)
{
#define COPY_REG(x) dst->x = src->x
    COPY_REG(a0);
    COPY_REG(a1);
    COPY_REG(a2);
    COPY_REG(a3);
    COPY_REG(a4);
    COPY_REG(a5);
    COPY_REG(a6);
    COPY_REG(a7);
#undef COPY_REG
}

#define UPATCH_INSN_LEN 8
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

/*
 * On RISC-V, there must be 3 instructors(12 bytes) to jump to
 * arbitrary address. The core upatch-manage limit jump instructor
 * to one long(8 bytes), for us is +-2G range.
 */
unsigned long get_new_insn(unsigned long old_addr, unsigned long new_addr)
{
    unsigned long offset;
    unsigned int insn0, insn4;

    offset = new_addr - old_addr;
    offset += (offset & 0x800) << 1;
    insn0 = 0xf97 | (offset & 0xfffff000);      // auipc t6, off[20]
    insn4 = 0xf8067 | ((offset & 0xfff) << 20); // jalr zero, off[12](t6)
    return (unsigned long)(insn0 | ((unsigned long)insn4 << 32));
}
