/* SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause) */
/* Copyright (c) 2023 Longjun Luo. */
#ifndef _UPATCH_MANAGER_H
#define _UPATCH_MANAGER_H

#define UPATCH_MAX_PATCH_ENTITY 10240

struct elf_process {
    unsigned long ino;
    int pid;
};

#endif /* _UPATCH_MANAGER_H */
