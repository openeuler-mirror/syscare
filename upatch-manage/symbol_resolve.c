// SPDX-License-Identifier: GPL-2.0
/*
 * resolve UND symbol in target or VMA so
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

#include "symbol_resolve.h"

#include <linux/fs.h>
#include <linux/mm.h>

#include "target_entity.h"

#include "patch_load.h"
#include "arch/patch_load.h"
#include "kernel_compat.h"
#include "util.h"

static inline bool is_same_name(const char *name, const char *name2)
{
    return strcmp(name, name2) == 0;
}

static unsigned long resolve_from_patch(const struct running_elf *relf,
    const char *name, Elf_Sym *patch_sym)
{
    const struct target_metadata *elf = relf->meta;
    unsigned long elf_addr = 0;

    if (!elf) {
        return 0;
    }

    if (!patch_sym->st_value) {
        return 0;
    }

    elf_addr = relf->vma_start_addr + patch_sym->st_value;
    log_debug("found symbol '%s' from patch at 0x%lx\n", name, elf_addr);

    return elf_addr;
}

/* handle external object, we need get it's address, used for R_X86_64_REX_GOTPCRELX */
static unsigned long resolve_from_rela_dyn(const struct running_elf *relf,
    const char *name, Elf_Sym *patch_sym)
{
    const struct target_metadata *elf = relf->meta;
    Elf_Sym *dynsym = elf->dynsym;
    Elf_Rela *rela_dyn = elf->rela_dyn;
    unsigned int i;
    const char *sym_name;
    void __user *sym_addr;
    unsigned long elf_addr;

    if (!dynsym || !rela_dyn || !elf->dynstr) {
        return 0;
    }

    for (i = 0; i < elf->rela_dyn_num; i++) {
        unsigned long sym_idx = ELF_R_SYM(rela_dyn[i].r_info);
        if (!sym_idx) {
            continue;
        }

        /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
        sym_name = elf->dynstr + dynsym[sym_idx].st_name;

        if (!is_same_name(sym_name, name)) {
            continue;
        }

        /* for executable file, r_offset is virtual address of GOT table */
        sym_addr = (void *)(relf->vma_start_addr + rela_dyn[i].r_offset);
        elf_addr = insert_got_table(relf->load_info, ELF_R_TYPE(rela_dyn[i].r_info), sym_addr);
        log_debug("found symbol '%s' from '.rela.dyn' at 0x%lx, ret=0x%lx\n",
            sym_name, (unsigned long)sym_addr, elf_addr);

        return elf_addr;
    }

    return 0;
}

static unsigned long resolve_from_rela_plt(const struct running_elf *relf,
    const char *name, Elf_Sym *patch_sym)
{
    const struct target_metadata *elf = relf->meta;
    Elf_Sym *dynsym = elf->dynsym;
    Elf_Rela *rela_plt = elf->rela_plt;
    unsigned int i;
    const char *sym_name;
    void __user *sym_addr;
    unsigned long elf_addr = 0;

    if (!dynsym || !rela_plt || !elf->dynstr) {
        return 0;
    }

    for (i = 0; i < elf->rela_plt_num; i++) {
        unsigned long sym_idx = ELF_R_SYM(rela_plt[i].r_info);
        unsigned long sym_type = ELF_ST_TYPE(dynsym[sym_idx].st_info);
        if (!sym_idx) {
            continue;
        }

        if ((sym_type != STT_FUNC) &&
            (sym_type != STT_TLS) &&
            (sym_type != STT_NOTYPE)) {
            continue;
        }

        sym_name = elf->dynstr + dynsym[sym_idx].st_name;
        if (!is_same_name(sym_name, name)) {
            continue;
        }

        /* for executable file, r_offset is virtual address of PLT table */
        sym_addr = (void *)(relf->vma_start_addr + rela_plt[i].r_offset);
        elf_addr = insert_plt_table(relf->load_info, ELF_R_TYPE(rela_plt[i].r_info), sym_addr);
        if (!elf_addr) {
            return 0;
        }
        log_debug("found symbol '%s' from '.rela.plt' at 0x%lx, ret=0x%lx\n",
            sym_name, (unsigned long)sym_addr, elf_addr);
        return elf_addr;
    }

    return 0;
}

// get symbol address from .dynsym
static unsigned long resolve_from_dynsym(const struct running_elf *relf, const char *name)
{
    const struct target_metadata *elf = relf->meta;
    Elf_Sym *dynsym = elf->dynsym;
    unsigned int i;
    const char *sym_name;
    void __user *sym_addr;
    unsigned long elf_addr = 0;

    if (!dynsym) {
        return 0;
    }

    for (i = 0; i < elf->dynsym_num; i++) {
        /* only need the st_value that is not 0 */
        if (dynsym[i].st_value == 0) {
            continue;
        }

        sym_name = elf->dynstr + dynsym[i].st_name;

        /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
        if (!is_same_name(sym_name, name)) {
            continue;
        }

        sym_addr = (void *)(relf->vma_start_addr + dynsym[i].st_value);
        elf_addr = insert_got_table(relf->load_info, 0, sym_addr);
        log_debug("found symbol '%s' from '.dynsym' at 0x%lx, ret=0x%lx\n",
            sym_name, (unsigned long)sym_addr, elf_addr);
        return elf_addr;
    }

    return 0;
}

static u32 sysv_hash(const unsigned char *name)
{
    u32 h = 0;
    u32 g = 0;
    for (; *name; name++) {
        h = (h << 4) + *name;
        g = h & 0xf0000000;
        if (g) {
            h ^= g >> 24;
        }
        h &= ~g;
    }
    return h;
}

static u32 gnu_hash(const char *name)
{
    u32 h = 5381;
    for (; *name; name++) {
        h += h * 32 + *name;
    }
    return h;
}

struct dynamic_shared_object {
    Elf_Ehdr ehdr;
    Elf_Shdr *shdrs;
    Elf_Dyn *dynamic;
    Elf_Sym *dynsym;
    Elf_Half *versions;
    Elf_Word *sysv_hash_buf;
    Elf_Word *gnu_hash_buf;
    char *strtab;

    Elf_Word dynstr_idx;
    unsigned int dynnum;
    unsigned int symnum;
};

static void free_shared_object(struct dynamic_shared_object *so)
{
    VFREE_CLEAR(so->shdrs);
    VFREE_CLEAR(so->dynamic);
    VFREE_CLEAR(so->dynsym);
    VFREE_CLEAR(so->versions);
    VFREE_CLEAR(so->sysv_hash_buf);
    VFREE_CLEAR(so->gnu_hash_buf);
    VFREE_CLEAR(so->strtab);
}

static int parse_shared_object(struct file *file, struct dynamic_shared_object *so)
{
    int ret = 0;
    Elf_Shdr *shdr;
    unsigned long i;

    // read elf header
    ret = kernel_read(file, &so->ehdr, sizeof(Elf_Ehdr), 0);
    if (ret != sizeof(Elf_Ehdr)) {
        log_err("failed to read elf header, ret=%d\n", ret);
        return -ENOEXEC;
    }

    // read section headers
    so->shdrs = vmalloc_read(file, so->ehdr.e_shoff, so->ehdr.e_shentsize * so->ehdr.e_shnum);
    if (IS_ERR(so->shdrs)) {
        ret = PTR_ERR(so->shdrs);
        log_err("failed to read section header, ret=%d\n", ret);
        return ret;
    }

    // Find the dynamic table and dynamic symbol table
    for (i = 0; i < so->ehdr.e_shnum; ++i) {
        shdr = &so->shdrs[i];
        switch (shdr->sh_type) {
            case SHT_DYNSYM:
                so->dynsym = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                so->symnum = shdr->sh_size / shdr->sh_entsize;
                so->dynstr_idx = shdr->sh_link;
                break;
            case SHT_DYNAMIC:
                so->dynamic = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                so->dynnum = shdr->sh_size / shdr->sh_entsize;
                break;
            case SHT_HASH:
                so->sysv_hash_buf = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                break;
            case SHT_GNU_HASH:
                so->gnu_hash_buf = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
                break;
            default:
                break;
        }
    }

    if (IS_ERR(so->dynsym)) {
        ret = PTR_ERR(so->dynsym);
        log_err("failed to read dynamic symbol table, ret=%d\n", ret);
        goto fail;
    }

    if (IS_ERR(so->dynamic)) {
        ret = PTR_ERR(so->dynamic);
        log_err("failed to read dynamic table, ret=%d\n", ret);
        goto fail;
    }

    if (IS_ERR(so->sysv_hash_buf)) {
        ret = PTR_ERR(so->sysv_hash_buf);
        log_err("failed to read sysv hash table, ret=%d\n", ret);
        goto fail;
    }

    if (IS_ERR(so->gnu_hash_buf)) {
        ret = PTR_ERR(so->gnu_hash_buf);
        log_err("failed to read gnu hash table, ret=%d\n", ret);
        goto fail;
    }

    if (!so->sysv_hash_buf && !so->gnu_hash_buf) {
        log_debug("shared object doesn't have symbol hash table!\n");
    }

    // read the version table for symbols
    for (i = 1; i < so->dynnum; ++i) {
        if (so->dynamic[i].d_tag != DT_VERSYM) {
            continue;
        }
        so->versions = vmalloc_read(file, so->dynamic[i].d_un.d_val, sizeof(Elf_Half) * so->symnum);
        if (IS_ERR(so->versions)) {
            ret = PTR_ERR(so->versions);
            log_err("failed to read version table, ret=%d\n", ret);
            goto fail;
        }
        break;
    }

    // Read the string table for symbols
    shdr = &so->shdrs[so->dynstr_idx];
    so->strtab = vmalloc_read(file, shdr->sh_offset, shdr->sh_size);
    if (IS_ERR(so->strtab)) {
        ret = PTR_ERR(so->strtab);
        log_err("failed to read symbol string table, ret=%d\n", ret);
        goto fail;
    }

    return 0;

fail:
    free_shared_object(so);
    return ret;
}

u32 g_hidden_version_sym_idx;
static bool is_sym_ok(struct dynamic_shared_object *so, u32 symidx, const char *name)
{
    char *sym_name = so->strtab + so->dynsym[symidx].st_name;
    if (!is_same_name(name, sym_name)) {
        return false;
    }

    // only glibc .so will have version symbol, other so->versions could be NULL
    // if found hidden version which is xxx@GLIBC_n.nn we keep the first found one and search default version sym.
    if (so->versions && (so->versions[symidx] & VERSYM_HIDDEN)) {
        if (!g_hidden_version_sym_idx) {
            g_hidden_version_sym_idx = symidx;
        }
        return false;
    } else {
        // if we found default symbol, which is xxx@GLIBC_n.nn, just use it and ignore hidden version
        return true;
    }
}

// return symbol index if found, 0 for not found.
static unsigned long search_by_gnu_hash_table(struct dynamic_shared_object *so, const char *name)
{
    const u32 nbuckets = so->gnu_hash_buf[0];
    const u32 symoffset = so->gnu_hash_buf[1];
    const u32 bloom_size = so->gnu_hash_buf[2];
    const u32 bloom_shift = so->gnu_hash_buf[3];
    const bloom_t *bloom = (void*)&so->gnu_hash_buf[4];
    const u32 *buckets = (void*)&bloom[bloom_size];
    const u32 *chain = &buckets[nbuckets];

    const u32 *hashval;
    u32 hash;
    u32 hash2;
    bloom_t bloom_word;
    bloom_t mask;
    u32 symidx;

    hash = gnu_hash(name);
    hash2 = hash >> bloom_shift;

    // Test the Bloom filter
    mask = ((bloom_t)1 << (hash % ELF_BITS)) | ((bloom_t)1 << (hash2 % ELF_BITS));

    bloom_word = bloom[(hash / ELF_BITS) % bloom_size];
    if ((bloom_word & mask) != mask) {
        return 0;
    }

    symidx = buckets[hash % nbuckets];
    if (symidx < symoffset) {
        return 0;
    }

    hashval = &chain[symidx - symoffset];

    for (hash |= 1; ; symidx++) {
        hash2 = *hashval;
        hashval++;
        if (hash == (hash2 | 1) && is_sym_ok(so, symidx, name)) {
            return symidx;
        }

        if (hash2 & 1) {
            break;
        }
    }

    return 0;
}

// return symbol index if found, 0 for not found.
static unsigned long search_by_sysv_hash_table(struct dynamic_shared_object *so, const char *name)
{
    const u32 nbucket = so->sysv_hash_buf[0];
    const u32* bucket = &so->sysv_hash_buf[2];
    const u32* chain = &so->sysv_hash_buf[nbucket];

    const u32 hash = sysv_hash(name);
    u32 i;

    for (i = bucket[hash % nbucket]; i; i = chain[i]) {
        if (is_sym_ok(so, i, name)) return i;
    }

    return 0;
}

static unsigned long search_by_iterate_symtab(struct dynamic_shared_object *so, const char *name)
{
    u32 i;
    for (i = 1; i < so->symnum; ++i) {
        if (is_sym_ok(so, i, name)) return i;
    }
    return 0;
}

static unsigned long find_sym_st_value_in_elf(struct file *file, const char *name, char *type)
{
    const char *file_name = file->f_path.dentry->d_name.name;
    Elf_Sym *sym = NULL;
    unsigned long offset = 0;
    unsigned long symidx = 0;
    struct dynamic_shared_object so = {0};

    log_debug("'%s'\t search in file %s\n", name, file_name);

    if (parse_shared_object(file, &so)) {
        log_err("Failed to parse %s\n", file_name);
        return 0;
    }

    g_hidden_version_sym_idx = 0;
    if (so.gnu_hash_buf) {
        // gnu hash is faster to search symbol
        symidx = search_by_gnu_hash_table(&so, name);
    } else if (so.sysv_hash_buf) {
        symidx = search_by_sysv_hash_table(&so, name);
    } else {
        symidx = search_by_iterate_symtab(&so, name);
    }

    if (!symidx && g_hidden_version_sym_idx) {
        // we only found one hidden version, use it.
        log_debug("[%d]: %s is hidden version!\n", g_hidden_version_sym_idx, name);
        symidx = g_hidden_version_sym_idx;
    }

    if (!symidx) {
        goto out;
    }

    sym = &so.dynsym[symidx];

    if (sym->st_shndx == SHN_UNDEF) {
        log_debug("symbol '%s' is UND, skipped\n", name);
        goto out;
    }

    // check symbol type
    if (!(1 << (ELF_ST_TYPE(sym->st_info)) & OK_TYPES)) {
        log_debug("symbol '%s' type is %d, skipped\n", name, ELF_ST_TYPE(sym->st_info));
        goto out;
    }

    // check symbol bind
    if (!(1 << (ELF_ST_BIND(sym->st_info)) & OK_BINDS)) {
        log_debug("symbol '%s' bind is %d, skipped\n", name, ELF_ST_BIND(sym->st_info));
        goto out;
    }

    offset = sym->st_value;
    *type = ELF_ST_TYPE(sym->st_info);

out:
    if (offset == 0) {
        log_debug("cannot find symbol '%s' in '%s'\n", name, file_name);
    } else {
        log_debug("'%s'\t offset 0x%lx in %s\n", name, offset, file_name);
    }
    free_shared_object(&so);
    return offset;
}

// Check if the current VMA is the text segment of shared object and is not the patched target itself
static bool is_vma_other_so_text(const struct running_elf *relf, struct vm_area_struct *vma)
{
    const char *file_path;
    bool is_so;
    if (!(vma->vm_file && (vma->vm_flags & VM_EXEC))) {
        return false;
    }

    file_path = vma->vm_file->f_path.dentry->d_name.name;
    if (!file_path) {
        return false;
    }

    is_so = strstr(file_path, ".so") != NULL;
    if (!is_so) {
        return false;
    }

    if (strcmp(file_path, relf->meta->file_name) == 0) {
        return false;
    }

    return true;
}

/* Caller must hold mm->mmap_lock */
static unsigned long search_base_addr(struct vm_area_struct *vma)
{
    unsigned long base_addr = vma->vm_start;
    struct vm_area_struct *search_vma;
    struct upatch_vma_iter vmi;

    // A file could be map into multiple VMA, find the first one
    upatch_vma_iter_set(&vmi, vma);
    while ((search_vma = upatch_vma_prev(&vmi))) {
        if (!search_vma->vm_file) {
            continue;
        }
        if (search_vma->vm_file->f_inode->i_sb->s_dev == vma->vm_file->f_inode->i_sb->s_dev &&
            search_vma->vm_file->f_inode->i_ino == vma->vm_file->f_inode->i_ino) {
            base_addr = search_vma->vm_start;
        }
    }

    return base_addr;
}

// Search all loaded so in current VMA, read the symbol table of so and find symbol offset
// Then combine with the so loaded base address, we can get the symbol loaded address
static unsigned long resolve_from_vma_so(const struct running_elf *relf, const char *symbol_name)
{
    struct vm_area_struct *vma;
    struct mm_struct *mm = current->mm;
    struct upatch_vma_iter vmi;

    unsigned long base_addr;
    unsigned long sym_addr = 0;
    unsigned long elf_addr = 0;
    char type;

    if (!mm) {
        return 0;
    }

    mmap_read_lock(mm);
    upatch_vma_iter_init(&vmi, mm);
    while ((vma = upatch_vma_next(&vmi))) {
        if (!is_vma_other_so_text(relf, vma)) {
            continue;
        }

        // Search for the symbol in the shared object
        sym_addr = find_sym_st_value_in_elf(vma->vm_file, symbol_name, &type);
        if (sym_addr == 0) {
            continue;
        }

        base_addr = search_base_addr(vma);
        sym_addr += base_addr;
        if ((type & STT_FUNC) || (type & STT_IFUNC)) {
            elf_addr = setup_jmp_table(relf->load_info, sym_addr, type == STT_IFUNC);
        } else {
            elf_addr = setup_got_table(relf->load_info, sym_addr, 0);
        }
        log_debug("found symbol '%s' from shared object at 0x%lx (base 0x%lx), ret=0x%lx\n",
            symbol_name, sym_addr, base_addr, elf_addr);
        break;
    }
    mmap_read_unlock(mm);

    return elf_addr;
}

static unsigned long resolve_from_symtab(const struct running_elf *relf, const char *name)
{
    const struct target_metadata *elf = relf->meta;
    Elf_Sym *sym = elf->symtab;
    unsigned int i;
    const char *sym_name;
    unsigned long elf_addr;

    if (!sym || !elf->strtab) {
        return 0;
    }

    for (i = 0; i < elf->symtab_num; i++) {
        if (sym[i].st_shndx == SHN_UNDEF) {
            continue;
        }
        sym_name = elf->strtab + sym[i].st_name;

        if (is_same_name(sym_name, name)) {
            elf_addr = relf->vma_start_addr + sym[i].st_value;
            log_debug("found symbol '%s' from '.symtab' at 0x%lx\n", name, elf_addr);
            return elf_addr;
        }
    }

    return 0;
}

/*
 * Handle external UND symbol:
 * 1. use symbol address from .dynsym, but most of its address is still undefined
 * 2. use address from PLT/GOT, problems are:
 * 3. read symbol from library that is loaded into VMA for the new called sym in .so
 */
unsigned long resolve_symbol(const struct running_elf *relf, const char *name, Elf_Sym patch_sym)
{
    unsigned long elf_addr = 0;

    if (!elf_addr) {
        elf_addr = resolve_from_vma_so(relf, name);
    }

    if (!elf_addr) {
        elf_addr = resolve_from_rela_plt(relf, name, &patch_sym);
    }

    /* resolve from got */
    if (!elf_addr) {
        elf_addr = resolve_from_rela_dyn(relf, name, &patch_sym);
    }

    if (!elf_addr) {
        elf_addr = resolve_from_dynsym(relf, name);
    }

    if (!elf_addr) {
        elf_addr = resolve_from_symtab(relf, name);
    }

    if (!elf_addr) {
        elf_addr = resolve_from_patch(relf, name, &patch_sym);
    }

    if (!elf_addr) {
        log_err("failed to resolve undefined symbol '%s'\n", name);
    }
    return elf_addr;
}