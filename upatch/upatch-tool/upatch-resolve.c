// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <fcntl.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <stdbool.h>

#include <sys/types.h>
#include <sys/stat.h>
#include <sys/mman.h>

#include <linux/elf.h>

static int read_from_offset(int fd, void **buf, int len, loff_t offset)
{
    int ret = -1;
    size_t size;

    *buf = malloc(len);
    if (*buf == NULL) {
        printf("malloc failed \n");
        goto out;
    }

    size = pread(fd, *buf, len, offset);
    if (size == -1) {
        ret = -errno;
        printf("read file failed - %d \n", -ret);
        goto out;
    }
    
    ret = 0;
out:
    return ret;
}

static bool inline streql(const char *a, const char *b)
{
    return strlen(a) == strlen(b) && !strncmp(a, b, strlen(a));
}

static int __partly_resolve_patch(Elf64_Sym *sym, const char *name,
    Elf64_Sym *binary_symtab, int binary_symnum, char *binary_strtab)
{
    int i;

    for (i = 1; i < binary_symnum; i ++) {
        Elf64_Sym *binary_sym = &binary_symtab[i];
        const char *binary_name;
        /* No need to handle section symbol */
        if (ELF_ST_TYPE(binary_sym->st_info) == STT_SECTION)
            continue;
        
        binary_name = binary_strtab + binary_sym->st_name;
        if (!streql(name, binary_name))
            continue;
        
        /* leave it to be handled in running time */
        if (binary_sym->st_shndx == SHN_UNDEF)
            continue;
        
        sym->st_shndx = SHN_LIVEPATCH;
        sym->st_info = binary_sym->st_info;
        sym->st_other = binary_sym->st_other;
        sym->st_value = binary_sym->st_value;
        sym->st_size = binary_sym->st_size;
        printf("found unresolved symbol %s at 0x%lx \n", name, (unsigned long)sym->st_value);
    }

    return 0;
}

static int partly_resolve_patch(int patch_fd, Elf64_Sym *binary_symtab,
    int binary_symnum, char *binary_strtab)
{
    int ret, i;
    int strindex, symindex = -1;
    Elf64_Ehdr *hdr = NULL;
    Elf64_Shdr *shdrs = NULL;
    Elf64_Sym *symtab = NULL;
    char *strtab;
    struct stat stat;

    ret = fstat(patch_fd, &stat);
    if (ret == -1) {
        ret = -errno;
        printf("fstat failed - %d \n", -ret);
        goto out;
    }

    hdr = mmap(NULL, stat.st_size,
        PROT_READ | PROT_WRITE, MAP_SHARED, patch_fd, 0);
    if (hdr == MAP_FAILED) {
        ret = -errno;
        printf("mmap failed - %d \n", -ret);
        goto out;
    }

    shdrs = (void *)hdr + hdr->e_shoff;
    for (i = 1; i < hdr->e_shnum; i ++) {
        if (shdrs[i].sh_type == SHT_SYMTAB) {
            symindex = i;
            strindex = shdrs[symindex].sh_link;
            break;
        }
    }

    if (symindex == -1) {
        ret = -EINVAL;
        printf("no symbol table found in patch file \n");
        goto out;
    }

    symtab = (void *)hdr + shdrs[symindex].sh_offset;
    strtab = (void *)hdr + shdrs[strindex].sh_offset;
    for (i = 0; i < shdrs[symindex].sh_size / sizeof(Elf64_Sym); i ++) {
        Elf64_Sym *sym = &symtab[i];
        const char *name;
        
        /* No need to handle section symbol */
        if (ELF_ST_TYPE(sym->st_info) == STT_SECTION)
            continue;
        
        name = strtab + sym->st_name;
        if (sym->st_shndx == SHN_UNDEF) {
            ret = __partly_resolve_patch(sym, name,
                binary_symtab, binary_symnum, binary_strtab);
            if (ret)
                goto out;
        }
        
    }

    ret = 0;
out:
    if (hdr != MAP_FAILED) {
        msync(hdr, stat.st_size, MS_SYNC);
        munmap(hdr, stat.st_size);
    }  
    return ret;
}

int resolve_patch(const char *binary, const char *patch)
{
    Elf64_Ehdr hdr;
    Elf64_Shdr *shdrs = NULL;
    Elf64_Sym *symtab = NULL;

    int binary_fd = -1, patch_fd = -1;
    int ret = 0, i, index;
    loff_t offset, len;
    int symnum;
    ssize_t cnt;
    char *strtab;

    binary_fd = open(binary, O_RDONLY);
    if (binary_fd == -1) {
        ret = -errno;
        printf("open binary failed - %d \n", -ret);
        goto out;
    }

    patch_fd = open(patch, O_RDWR);
    if (patch_fd == -1) {
        ret = -errno;
        printf("open patch failed - %d \n", -ret);
        goto out;
    }

    offset = 0;
    len = sizeof(Elf64_Ehdr);
    cnt = pread(binary_fd, &hdr, len, offset);
    if (cnt == -1) {
        ret = -errno;
        printf("pread binary failed - %d \n", -ret);
        goto out;
    }

    offset = hdr.e_shoff;
    len = sizeof(Elf64_Shdr) * hdr.e_shnum;
    ret = read_from_offset(binary_fd, (void **)&shdrs, len, offset);
    if (ret)
        goto out;

    index = -1;
    for (i = 1; i < hdr.e_shnum; i ++) {
        if (shdrs[i].sh_type == SHT_SYMTAB) {
            index = i;
            break;
        }
    }

    if (index == -1) {
        ret = -EINVAL;
        printf("no symtab found \n");
        goto out;
    }

    len = shdrs[index].sh_size;
    symnum = len / sizeof(Elf64_Sym);
    offset = shdrs[index].sh_offset;
    ret = read_from_offset(binary_fd, (void **)&symtab, len, offset);
    if (ret)
        goto out;

    index = shdrs[index].sh_link;
    len = shdrs[index].sh_size;
    offset = shdrs[index].sh_offset;
    ret = read_from_offset(binary_fd, (void **)&strtab, len, offset);
    if (ret)
        goto out;

    ret = partly_resolve_patch(patch_fd, symtab, symnum, strtab);
    if (ret)
        goto out;

    ret = 0;
out:
    if (symtab)
        free(symtab);
    if (shdrs)
        free(shdrs);
    if (binary_fd != -1)
        close(binary_fd);
    if (patch_fd != -1)
        close(patch_fd);
    return ret;
}