// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/linkage.h>
#include <asm/page_types.h>

    .text
    .align PAGE_SIZE

SYM_CODE_START_NOALIGN(__execve_syscall)
    .code64
    mov     $0x3b, %eax
    syscall
    /* if execve failed, exit with its return value */
    mov     %eax, %edi
    mov     $0x3c, %eax
    syscall
SYM_CODE_END(__execve_syscall)

/* use exit syscall to pass errno */
SYM_CODE_START_NOALIGN(__exit_syscall)
    .code64
    mov     $0x3c, %eax
    syscall
SYM_CODE_END(__exit_syscall)