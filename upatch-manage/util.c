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

#include "util.h"

#include <linux/fs.h>
#include <linux/namei.h>
#include <linux/vmalloc.h>
#include <linux/elf.h>
#include <linux/err.h>
#include <linux/string.h>

#include "patch_entity.h"

bool is_elf_valid(Elf_Ehdr *ehdr, size_t len, bool is_patch)
{
    if (memcmp(ehdr->e_ident, ELFMAG, SELFMAG) != 0) {
        log_err("elf magic wrong!\n");
        return false;
    }

    if (!elf_check_arch(ehdr)) {
        log_err("elf_check_arch failed, e_machine = %d\n", ehdr->e_machine);
        return false;
    }

    if (!ehdr->e_shoff || !ehdr->e_shnum || !ehdr->e_shentsize) {
        log_err("file don't have section\n");
        return false;
    }

    if (ehdr->e_shentsize != sizeof(Elf_Shdr)) {
        log_err("e_shentsize is %d not %zu\n", ehdr->e_shentsize, sizeof(Elf_Shdr));
        return false;
    }

    if (ehdr->e_shstrndx > (ehdr->e_shnum - 1)) {
        log_err("e_shstrndx = %d, greater than e_shnum = %d\n",
            ehdr->e_shstrndx, ehdr->e_shnum);
        return false;
    }

    if (len < sizeof(Elf_Ehdr) || ehdr->e_shoff >= len ||
        ehdr->e_shnum * sizeof(Elf_Shdr) > len - ehdr->e_shoff) {
        log_err("len is %ld, not suitable with e_shnum %d and e_shoff %lld\n",
            (long int)len, ehdr->e_shnum, (long long)ehdr->e_shoff);
        return false;
    }

    if (is_patch) {
        if (ehdr->e_type != ET_REL) {
            log_err("patch is not REL format\n");
            return false;
        }
    } else {
        if ((ehdr->e_type != ET_EXEC) && (ehdr->e_type != ET_DYN)) {
            log_err("file is not exe or so\n");
            return false;
        }

        if (!ehdr->e_phoff || !ehdr->e_phnum || !ehdr->e_phentsize) {
            log_err("file don't have program header\n");
            return false;
        }
    }

    return true;
}

struct inode *path_inode(const char *file)
{
    struct path path;
    struct inode *inode;
    int ret = 0;

    ret = kern_path(file, LOOKUP_FOLLOW, &path);
    if (ret) {
        log_err("%s: cannot get inode of %s\n", __func__, file);
        return ERR_PTR(ret);
    }

    inode = path.dentry->d_inode;
    if (!inode) {
        log_err("%s: path inode is NULL, path = %s\n", __func__, file);
        return ERR_PTR(-ENOENT);
    }
    path_put(&path);
    return inode;
}

void *vmalloc_read(struct file *file, loff_t offset, size_t len)
{
    void *buff = NULL;
    ssize_t read = 0;

    if (!len) {
        return ERR_PTR(-EINVAL);
    }

    buff = vmalloc(len);
    if (!buff) {
        return ERR_PTR(-ENOMEM);
    }

    read = kernel_read(file, buff, len, &offset);
    if (read < 0) {
        vfree(buff);
        return ERR_PTR(read);
    }
    if (read != len) {
        vfree(buff);
        return ERR_PTR(-EIO);
    }

    return buff;
}