// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#include <linux/linkage.h>
#include <asm/unistd.h>

.text

#ifndef SYM_CODE_START
#define SYM_CODE_START(name)                            \
        SYM_START(name, SYM_L_GLOBAL, SYM_A_ALIGN)
#endif

#ifndef SYM_CODE_END
#define SYM_CODE_END(name)                              \
        SYM_END(name, SYM_T_NONE)
#endif

#ifndef SYM_START
#define SYM_START(name, linkage, align...)              \
        SYM_ENTRY(name, linkage, align)
#endif

#ifndef SYM_END
#define SYM_END(name, sym_type)                         \
        .type name sym_type ASM_NL                      \
        .size name, .-name
#endif

#ifndef SYM_ENTRY
#define SYM_ENTRY(name, linkage, align...)              \
        linkage(name) ASM_NL                            \
        align ASM_NL                                    \
        name:
#endif

#ifndef ASM_NL
#define ASM_NL           ;
#endif

#ifndef SYM_T_NONE
#define SYM_T_NONE                              STT_NOTYPE
#endif

#ifndef SYM_L_GLOBAL
#define SYM_L_GLOBAL(name)                      .globl name
#endif

#ifndef SYM_A_ALIGN
#define SYM_A_ALIGN                             ALIGN
#endif

SYM_CODE_START(__execve_syscall)
    mov     x8, __NR_execve
    svc     #0
    mov     x8, __NR_exit
    svc     #0
SYM_CODE_END(__execve_syscall)

SYM_CODE_START(__exit_syscall)
    mov     x8, __NR_exit
    svc     #0
SYM_CODE_END(__exit_syscall)
