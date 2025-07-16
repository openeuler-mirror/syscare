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
#include <linux/module.h>

#include <linux/err.h>
#include <linux/fs.h>
#include <linux/namei.h>
#include <linux/slab.h>
#include <linux/vmalloc.h>

static const char* MODULE_NAME = THIS_MODULE->name;

#define log_err(fmt, args...)    pr_err_ratelimited("%s: " fmt, MODULE_NAME, ##args)
#define log_warn(fmt, args...)   pr_warn_ratelimited("%s: " fmt, MODULE_NAME, ##args)
#define log_info(fmt, args...)   pr_info_ratelimited("%s: " fmt, MODULE_NAME, ##args)
#define log_debug(fmt, args...)  pr_debug_ratelimited("%s: " fmt, MODULE_NAME, ##args)

/*
 * Alloc buffer and read file content
 * @param path: file
 * @param offset: file offset
 * @param len: read length
 * @return buffer pointer
 */
static inline void *vmalloc_read(struct file *file, loff_t offset, size_t len)
{
    void *buff;
    ssize_t ret;

    if (unlikely(!file || !len)) {
        return ERR_PTR(-EINVAL);
    }

    buff = vmalloc(len);
    if (unlikely(!buff)) {
        return ERR_PTR(-ENOMEM);
    }

    ret = kernel_read(file, buff, len, &offset);
    if (unlikely(ret < 0)) {
        vfree(buff);
        return ERR_PTR(ret);
    }

    if (unlikely(ret != len)) {
        vfree(buff);
        return ERR_PTR(-EIO);
    }

    return buff;
}

static inline void *vmalloc_copy_user(void __user *addr, size_t offset, size_t size)
{
    void __user *uaddr;
    void *kaddr;

    if (unlikely(!addr || !size)) {
        return ERR_PTR(-EINVAL);
    }

    if (unlikely(check_add_overflow((unsigned long)addr, offset, (unsigned long*)&uaddr))) {
        return ERR_PTR(-EINVAL);
    }

    if (unlikely(!access_ok(uaddr, size))) {
        return ERR_PTR(-EFAULT);
    }

    kaddr = vmalloc(size);
    if (unlikely(!kaddr)) {
        return ERR_PTR(-ENOMEM);
    }

    if (unlikely(copy_from_user(kaddr, uaddr, size))) {
        vfree(kaddr);
        return ERR_PTR(-EFAULT);
    }

    return kaddr;
}

/*
 * Free valloc() allocated memory safely
 * @param addr: memory address
 * @return void
 */
static inline void vfree_safe(const void *addr)
{
    if (likely(addr && !IS_ERR(addr))) {
        vfree(addr);
    }
}

#define VFREE_CLEAR(ptr) do { vfree_safe((ptr)); (ptr) = NULL; } while (0)

/*
 * Free kalloc() allocated memory safely
 * @param addr: memory address
 * @return void
 */
static inline void kfree_safe(const void *addr)
{
    if (likely(addr && !IS_ERR(addr))) {
        kfree(addr);
    }
}

#define KFREE_CLEAR(ptr) do { kfree_safe((ptr)); (ptr) = NULL; } while (0)

static inline bool is_valid_elf(Elf_Ehdr *ehdr, size_t len)
{
    return ehdr && len >= sizeof(Elf_Ehdr) &&
        ehdr->e_ident[EI_MAG0] == ELFMAG0 && ehdr->e_ident[EI_MAG1] == ELFMAG1 &&
        ehdr->e_ident[EI_MAG2] == ELFMAG2 && ehdr->e_ident[EI_MAG3] == ELFMAG3 &&
        elf_check_arch(ehdr) &&
        ehdr->e_shoff && ehdr->e_shoff < len &&
        ehdr->e_shnum && ehdr->e_shstrndx < ehdr->e_shnum && ehdr->e_shentsize == sizeof(Elf_Shdr) &&
        ehdr->e_shnum * ehdr->e_shentsize <= len - ehdr->e_shoff &&
        (!ehdr->e_phoff || (ehdr->e_phoff < len && ehdr->e_phnum && ehdr->e_phentsize == sizeof(Elf_Phdr) &&
        ehdr->e_phnum * ehdr->e_phentsize <= len - ehdr->e_phoff));
}

static inline bool is_valid_patch(Elf_Ehdr *ehdr, size_t len)
{
    return is_valid_elf(ehdr, len) && (ehdr->e_type == ET_REL);
}

static inline bool is_valid_target(Elf_Ehdr *ehdr, size_t len)
{
    return is_valid_elf(ehdr, len) && ((ehdr->e_type == ET_EXEC) || (ehdr->e_type == ET_DYN)) && ehdr->e_phnum;
}

static inline bool is_valid_str(const char *strtab, size_t strtab_len, size_t offset)
{
    const char *start;
    size_t remain_len;

    if (unlikely(offset >= strtab_len - 1)) {
        return false;
    }

    start = strtab + offset;
    remain_len = strtab_len - offset;
    return memchr(start, '\0', remain_len) != NULL;
}

static inline const char *get_string_at(const char *strtab, size_t strtab_len, size_t offset)
{
    return is_valid_str(strtab, strtab_len, offset) ? strtab + offset : NULL;
}

static inline struct inode *get_path_inode(const char *file)
{
    struct path path;
    struct inode *inode;

    if (unlikely(!file || kern_path(file, LOOKUP_FOLLOW, &path))) {
        return NULL;
    }

    inode = igrab(path.dentry->d_inode); // will increase inode refcnt, need call iput after use

    path_put(&path);
    return inode;
}

#endif // _UPATCH_MANAGE_UTIL_H
