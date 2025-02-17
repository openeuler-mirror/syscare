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

#include <errno.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <asm/unistd.h>
#include <sys/ptrace.h>
#include <sys/wait.h>

#include "upatch-common.h"
#include "upatch-ptrace.h"

/* process's memory access */
int upatch_process_mem_read(struct upatch_process *proc, unsigned long src,
    void *dst, size_t size)
{
    ssize_t r = pread(proc->memfd, dst, size, (off_t)src);

    return r != (ssize_t)size ? -1 : 0;
}

static int upatch_process_mem_write_ptrace(struct upatch_process *proc,
    const void *src, unsigned long dst, size_t size)
{
    long ret;

    while (ROUND_DOWN(size, sizeof(long)) != 0) {
        ret = ptrace(PTRACE_POKEDATA, proc->pid, dst, *(const unsigned long *)src);
        if (ret) {
            return -1;
        }
        dst += sizeof(long);
        src += sizeof(long);
        size -= sizeof(long);
    }

    if (size) {
        long tmp;

        tmp = ptrace(PTRACE_PEEKDATA, proc->pid, dst, NULL);
        if (tmp == -1 && errno) {
            return -1;
        }
        memcpy(&tmp, src, size);

        ret = ptrace(PTRACE_POKEDATA, proc->pid, dst, tmp);
        if (ret) {
            return -1;
        }
    }

    return 0;
}

int upatch_process_mem_write(struct upatch_process *proc, const void *src,
    unsigned long dst, size_t size)
{
    static int use_pwrite = 1;
    ssize_t w;

    if (use_pwrite) {
        w = pwrite(proc->memfd, src, size, (off_t)dst);
    }
    if (!use_pwrite || (w == -1 && errno == EINVAL)) {
        use_pwrite = 0;
        return upatch_process_mem_write_ptrace(proc, src, dst, size);
    }

    return w != (ssize_t)size ? -1 : 0;
}

static struct upatch_ptrace_ctx* upatch_ptrace_ctx_alloc(
    struct upatch_process *proc)
{
    struct upatch_ptrace_ctx *p;

    p = malloc(sizeof(*p));
    if (!p) {
        return NULL;
    }

    memset(p, 0, sizeof(*p));

    p->execute_until = 0UL;
    p->running = 1;
    p->proc = proc;

    INIT_LIST_HEAD(&p->list);
    list_add(&p->list, &proc->ptrace.pctxs);

    return p;
}

int upatch_ptrace_attach_thread(struct upatch_process *proc, int tid)
{
    struct upatch_ptrace_ctx *pctx = upatch_ptrace_ctx_alloc(proc);
    if (pctx == NULL) {
        log_error("Failed to alloc ptrace context");
        return -1;
    }

    pctx->pid = tid;
    log_debug("Attaching to %d...", tid);

    long ret = ptrace(PTRACE_ATTACH, tid, NULL, NULL);
    if (ret < 0) {
        log_error("Failed to attach thread, pid=%d, ret=%ld\n", tid, ret);
        return -1;
    }

    do {
        int status = 0;

        ret = waitpid(tid, &status, __WALL);
        if (ret < 0) {
            log_error("Failed to wait thread, tid=%d, ret=%ld\n", tid, ret);
            return -1;
        }

        /* We are expecting SIGSTOP */
        if (WIFSTOPPED(status) && WSTOPSIG(status) == SIGSTOP) {
            break;
        }

        /* If we got SIGTRAP because we just got out of execve, wait
         * for the SIGSTOP
         */
        if (WIFSTOPPED(status)) {
            status = (WSTOPSIG(status) == SIGTRAP) ? 0 : WSTOPSIG(status);
        } else if (WIFSIGNALED(status)) {
            /* Resend signal */
            status = WTERMSIG(status);
        }

        ret = ptrace(PTRACE_CONT, tid, NULL, (void *)(uintptr_t)status);
        if (ret < 0) {
            log_error("Failed to continue thread, tid=%d, ret=%ld\n", tid, ret);
            return -1;
        }
    } while (1);

    pctx->running = 0;

    log_debug("OK\n");
    return 0;
}

int wait_for_stop(struct upatch_ptrace_ctx *pctx, const void *data)
{
    long ret;

    int status = 0;
    int pid = (int)(uintptr_t)data ?: pctx->pid;
    log_debug("wait_for_stop(pctx->pid=%d, pid=%d)\n", pctx->pid, pid);

    while (1) {
        ret = ptrace(PTRACE_CONT, pctx->pid, NULL, (void *)(uintptr_t)status);
        if (ret < 0) {
            log_error("Cannot start tracee %d, ret=%ld\n", pctx->pid, ret);
            return -1;
        }

        ret = waitpid(pid, &status, __WALL);
        if (ret < 0) {
            log_error("Cannot wait tracee %d, ret=%ld\n", pid, ret);
            return -1;
        }

        if (WIFSTOPPED(status)) {
            if (WSTOPSIG(status) == SIGSTOP || WSTOPSIG(status) == SIGTRAP) {
                break;
            }
            status = WSTOPSIG(status);
            continue;
        }

        status = WIFSIGNALED(status) ? WTERMSIG(status) : 0;
    }

    return 0;
}

int upatch_ptrace_detach(struct upatch_ptrace_ctx *pctx)
{
    if (!pctx->pid) {
        return 0;
    }

    log_debug("Detaching from %d...", pctx->pid);
    long ret = ptrace(PTRACE_DETACH, pctx->pid, NULL, NULL);
    if (ret < 0) {
        log_error("Failed to detach from process, pid=%d, ret=%ld\n", pctx->pid, ret);
        return -errno;
    }
    log_debug("OK\n");

    pctx->running = 1;
    pctx->pid = 0;
    return 0;
}

long upatch_execute_remote(struct upatch_ptrace_ctx *pctx,
    const unsigned char *code, size_t codelen,
    struct user_regs_struct *pregs)
{
    return upatch_arch_execute_remote_func(pctx, code, codelen, pregs,
        wait_for_stop, NULL);
}

unsigned long upatch_mmap_remote(struct upatch_ptrace_ctx *pctx,
    unsigned long addr, size_t length, unsigned long prot,
    unsigned long flags, unsigned long fd, unsigned long offset)
{
    long ret;
    unsigned long res = 0;

    log_debug("mmap_remote: 0x%lx+%lx, %lx, %lx, %lu, %lx\n", addr, length,
        prot, flags, fd, offset);
    ret = upatch_arch_syscall_remote(pctx, __NR_mmap,
        (unsigned long)addr, length, prot, flags, fd, offset, &res);
    if (ret < 0) {
        return 0;
    }

    if (ret == 0 && res >= (unsigned long)-MAX_ERRNO) {
        errno = -(int)res;
        return 0;
    }

    return res;
}

int upatch_mprotect_remote(struct upatch_ptrace_ctx *pctx, unsigned long addr,
    size_t length, unsigned long prot)
{
    long ret;
    unsigned long res;

    log_debug("mprotect_remote: 0x%lx+%lx\n", addr, length);
    ret = upatch_arch_syscall_remote(pctx, __NR_mprotect,
        (unsigned long)addr, length, prot, 0, 0, 0, &res);
    if (ret < 0) {
        return -1;
    }

    if (ret == 0 && res >= (unsigned long)-MAX_ERRNO) {
        errno = -(int)res;
        return -1;
    }

    return 0;
}

int upatch_munmap_remote(struct upatch_ptrace_ctx *pctx, unsigned long addr,
    size_t length)
{
    long ret;
    unsigned long res;

    log_debug("munmap_remote: 0x%lx+%lx\n", addr, length);
    ret = upatch_arch_syscall_remote(pctx, __NR_munmap,
        (unsigned long)addr, length, 0, 0, 0, 0, &res);
    if (ret < 0) {
        return -1;
    }

    if (ret == 0 && res >= (unsigned long)-MAX_ERRNO) {
        errno = -(int)res;
        return -1;
    }

    return 0;
}
