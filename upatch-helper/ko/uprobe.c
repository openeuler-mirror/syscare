// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-helper kernel module
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

#include "uprobe.h"

#include <linux/fs.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/namei.h>
#include <linux/uprobes.h>
#include <linux/slab.h>

#include "log.h"
#include "map.h"
#include "context.h"
#include "records.h"
#include "cache.h"
#include "utils.h"

#ifdef __x86_64__
#define _reg_argv0 regs->di
#endif

#ifdef __aarch64__
#define _reg_argv0 regs->regs[0]
#endif

/* Uprobe private interface */
static inline char* read_user_str(char *dst, const char __user *src, size_t count)
{
    size_t len = strncpy_from_user(dst, src, (long)count);
    if (len <= 0) {
        pr_err("failed to read from user space\n");
        return NULL;
    }
    dst[len] = '\0';

    return dst;
}

static inline const char __user *new_user_str(const char *src, size_t len)
{
    unsigned long addr = vm_mmap(NULL, 0, len,
        PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);

    if (addr == 0) {
        pr_err_ratelimited("failed to alloc in userspace\n");
        return NULL;
    }

    if (copy_to_user((void *)addr, src, len) != 0) {
        pr_err_ratelimited("failed to write to userspace\n");
        (void)vm_munmap(addr, len);
        return NULL;
    }

    return (const char __user *)addr;
}

static inline const char *select_jump_path(const struct helper_record *record,
    const struct inode *inode)
{
    if (inode_equal(inode, record->exec_inode)) {
        return record->jump_path;
    }
    if (inode_equal(inode, record->jump_inode)) {
        return record->exec_path;
    }
    return NULL;
}

/* Uprobe public interface */
int handle_uprobe(struct uprobe_consumer *self, struct pt_regs *regs)
{
    const char __user *argv0 = (const char __user *)_reg_argv0;
    const char __user *new_argv0 = NULL;

    struct map *helper_map = get_helper_map();
    const struct helper_record *record = NULL;

    const char *elf_path = NULL;
    const char *jump_path = NULL;

    const struct inode *inode = NULL;
    char *path_buff = NULL;
    size_t path_len = 0;

    if ((argv0 == NULL) || (helper_context_count() == 0)) {
        return 0;
    }

    if (map_size(helper_map) == 0) {
        return 0;
    }

    path_buff = path_buf_alloc();
    if (path_buff == NULL) {
        pr_err_ratelimited("failed to alloc path cache\n");
        return 0;
    }

    elf_path = read_user_str(path_buff, argv0, PATH_MAX);
    if (elf_path == NULL) {
        pr_err_ratelimited("failed to read execve argument from userspace\n");
        path_buf_free(path_buff);
        return 0;
    }

    inode = path_inode(elf_path);
    if (inode == NULL) {
        path_buf_free(path_buff);
        return 0;
    }

    record = (const struct helper_record *)map_get(helper_map, inode);
    if (record == NULL) {
        pr_debug("record not found, elf_path=%s\n", elf_path);
        path_buf_free(path_buff);
        return 0;
    }

    jump_path = select_jump_path(record, inode);
    if (jump_path == NULL) {
        pr_err_ratelimited("failed to find jump path, elf_path=%s\n", elf_path);
        path_buf_free(path_buff);
        return 0;
    }
    path_len = strnlen(jump_path, PATH_MAX) + 1;
    pr_debug("[helped] elf_path=%s, jump_path=%s\n", elf_path, jump_path);

    new_argv0 = new_user_str(jump_path, path_len);
    if (new_argv0 == NULL) {
        pr_err_ratelimited("failed to write new execve argument\n");
        path_buf_free(path_buff);
        return 0;
    }

    path_buf_free(path_buff);

    // We won't free new allocated userspace memory
    // since it would be used by execve
    _reg_argv0 = (unsigned long)new_argv0;

    return 0; // always return 0, so that execve would never fail
}
