// SPDX-License-Identifier: GPL-2.0
/*
 * provide utils
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

#ifndef _UPATCH_MANAGE_UTIL_H
#define _UPATCH_MANAGE_UTIL_H

#include <linux/types.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>
#include <linux/printk.h>

#include <linux/elf.h>
#include <linux/module.h>

static const char* MODULE_NAME = THIS_MODULE->name;

#define log_err(fmt, args...)    pr_err("%s: " fmt, MODULE_NAME, ##args)
#define log_warn(fmt, args...)   pr_warn("%s: " fmt, MODULE_NAME, ##args)
#define log_info(fmt, args...)   pr_info("%s: " fmt, MODULE_NAME, ##args)
#define log_debug(fmt, args...)  pr_debug("%s: " fmt, MODULE_NAME, ##args)

/*
 * Alloc buffer and read file content
 * @param path: file
 * @param offset: file offset
 * @param len: read length
 * @return buffer pointer
 */
void *vmalloc_read(struct file *file, loff_t offset, size_t len);

/*
 * Free kalloc() allocated memory safely
 * @param addr: memory address
 * @return void
 */
static inline void kfree_safe(const void *addr)
{
    if (addr) {
        kfree(addr);
    }
}

#define KFREE_CLEAR(ptr) do { kfree_safe(ptr); ptr = NULL; } while (0)

/*
 * Free valloc() allocated memory safely
 * @param addr: memory address
 * @return void
 */
static inline void vfree_safe(const void *addr)
{
    if (addr) {
        vfree(addr);
    }
}

#define VFREE_CLEAR(ptr) do { vfree_safe(ptr); ptr = NULL; } while (0)

bool is_elf_valid(Elf_Ehdr *ehdr, size_t len, bool is_patch);

struct inode *path_inode(const char *file);

#endif // _UPATCH_MANAGE_UTIL_H
