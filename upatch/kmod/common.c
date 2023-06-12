// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/binfmts.h> /* for MAX_ARG_STRLEN */
#include <linux/slab.h>
#include <linux/elf.h>
#include <linux/fs.h>
#include <linux/mm.h>

#include "common.h"

/* Common used tool functions */
inline int copy_para_from_user(unsigned long addr, char *buf, size_t buf_len)
{
    size_t len;

    if (!buf || addr == 0)
        return -EINVAL;

    len = strnlen_user((void __user *)addr, MAX_ARG_STRLEN);
    if (len < 0 || len > buf_len)
        return -EOVERFLOW;

    if (copy_from_user(buf, (void __user *)addr, len))
        return -ENOMEM;

    return 0;
}

struct file *get_binary_file_from_addr(struct task_struct *task, unsigned long addr)
{
    struct vm_area_struct *vma = NULL;

    vma = find_vma(task->mm, addr);
    if (!vma)
        return NULL;

    if (!vma->vm_file)
        return NULL;

    return vma->vm_file;
}

int obtain_parameter_addr(char __user **pointer_array, char *name,
    unsigned long *addr, unsigned long *next_addr)
{
    int ret;
    unsigned long tmp;
    unsigned long addr_pointer, next_pointer;

    if (addr)
        *addr = 0;

    if (next_addr)
        *next_addr = 0;

    ret = obtain_parameter_pointer(pointer_array, name, &addr_pointer, &next_pointer);
    if (ret)
        return ret;

    if (addr && addr_pointer != 0
        && !get_user(tmp, (unsigned long __user *)addr_pointer))
        *addr = tmp;

    if (next_addr && next_pointer != 0
        && !get_user(tmp, (unsigned long __user *)next_pointer))
        *next_addr = tmp;

    return 0;
}

int obtain_parameter_pointer(char __user **pointer_array, char *name,
    unsigned long *addr_pointer, unsigned long *next_pointer)
{
    char *__buffer;
    unsigned long pointer_addr;
    size_t len = strlen(name);

    if (!pointer_array)
        return -EINVAL;

    __buffer = kmalloc(len + 1, GFP_KERNEL);
    if (!__buffer)
        return -ENOMEM;

    __buffer[len] = '\0';

    if (addr_pointer)
        *addr_pointer = 0;

    if (next_pointer)
        *next_pointer = 0;

    for (;;) {
        /* get pointer address first */
        if (get_user(pointer_addr, (unsigned long __user *)pointer_array))
            break;
        pointer_array ++;

        if (!(const char __user *)pointer_addr)
            break;

        if (copy_from_user(__buffer, (void __user *)pointer_addr, len))
            break;

        /* if not matched, continue */
        if (strncmp(__buffer, name, len))
            continue;

        pointer_array --;
        if (addr_pointer)
            *addr_pointer = (unsigned long)(unsigned long __user *)pointer_array;

        pointer_array ++;
        if (next_pointer)
            *next_pointer = (unsigned long)(unsigned long __user *)pointer_array;

        break;
    }

    if (__buffer)
        kfree(__buffer);

    return 0;
}

char __user **get_argv_from_regs(struct pt_regs *regs)
{
    unsigned long stack_pointer = user_stack_pointer(regs);
    return (void *)(stack_pointer + sizeof(unsigned long));
}

char __user **get_env_from_regs(struct pt_regs *regs)
{
    int argc;
    unsigned long stack_pointer = user_stack_pointer(regs);
    char __user **argv = get_argv_from_regs(regs);

    if (get_user(argc, (int *)stack_pointer)) {
        pr_err("unable to read argc from stack pointer \n");
        return NULL;
    }

    return (void *)((unsigned long)argv + (argc + 1) * sizeof(unsigned long));
}
