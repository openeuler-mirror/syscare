// SPDX-License-Identifier: GPL-2.0
/*
 * when user program hit uprobe trap and go into kernel, load patch into VMA
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

#include "patch_load.h"

#include <linux/fs.h>
#include <linux/mman.h>

#include "arch/patch_load.h"

#include "target_entity.h"
#include "process_entity.h"
#include "patch_entity.h"
#include "symbol_resolve.h"

#include "kernel_compat.h"
#include "util.h"

#define FLAG_LEN       3

#ifndef ARCH_SHF_SMALL
#define ARCH_SHF_SMALL 0
#endif

static void layout_sections(struct patch_context *ctx)
{
    static const unsigned long SECTION_FLAGS[][2] = {
        /* NOTE: all executable code must be the first section
         * in this array; otherwise modify the text_size
         * finder in the two loops below */
        { SHF_EXECINSTR | SHF_ALLOC, ARCH_SHF_SMALL },
        { SHF_ALLOC, SHF_WRITE | ARCH_SHF_SMALL },
        { SHF_RO_AFTER_INIT | SHF_ALLOC, ARCH_SHF_SMALL },
        { SHF_WRITE | SHF_ALLOC, ARCH_SHF_SMALL },
        { ARCH_SHF_SMALL | SHF_ALLOC, 0 }
    };

    size_t und_sym_num = ctx->patch->und_sym_num;
    size_t got_reloc_num = ctx->patch->got_reloc_num;

    struct patch_layout *layout = &ctx->layout;
    Elf_Half m;

    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Half shdr_num = ctx->ehdr->e_shnum;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab = ctx->buff + ctx->shstrtab_shdr->sh_offset;
    const char *sec_name;

    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];
        /* set all section address to their address of patch image */
        shdr->sh_addr = (Elf_Addr)ctx->buff + shdr->sh_offset;
        /* set all section entity size to invalid value */
        shdr->sh_entsize = ~0UL;
    }

    log_debug("patch layout:\n");
    for (m = 0; m < ARRAY_SIZE(SECTION_FLAGS); ++m) {
        for (i = 1; i < shdr_num; i++) {
            shdr = &shdrs[i];

            if ((shdr->sh_flags & SECTION_FLAGS[m][0]) != SECTION_FLAGS[m][0] ||
                (shdr->sh_flags & SECTION_FLAGS[m][1]) ||
                shdr->sh_entsize != ~0UL) {
                continue;
            }

            sec_name = shstrtab + shdr->sh_name;
            shdr->sh_entsize = ALIGN(layout->size, shdr->sh_addralign ?: 1);
            layout->size = shdr->sh_entsize + shdr->sh_size;

            log_debug("type[%02d] %-20s \tend at 0x%x\n", m, sec_name, layout->size);
        }
        switch (m) {
            case 0: /* executable */
                layout->table.off = ALIGN(layout->size, sizeof(unsigned long));
                layout->table.cur = 0;
                layout->table.max = und_sym_num * JMP_TABLE_ENTRY_MAX_SIZE + got_reloc_num * JMP_TABLE_GOT_ENTRY_SIZE;
                layout->size = PAGE_ALIGN(layout->table.off + layout->table.max);
                layout->text_end = layout->size;
                log_debug("\t\t%-20s \t0x%lx - 0x%x max size %d\n", "jmptable",
                    layout->table.off, layout->size, layout->table.max);
                break;
            case 1: /* RO: text and ro-data */
                layout->size = PAGE_ALIGN(layout->size);
                layout->ro_end = layout->size;
                break;
            case 2: /* RO after init */
                layout->size = PAGE_ALIGN(layout->size);
                layout->ro_after_init_end = layout->size;
                break;
            case 4: /* writable and small data */
                layout->size = PAGE_ALIGN(layout->size);
                break;
            default:
                break;
        }
    }
}

static int init_load_info(struct patch_context *ctx,
    const struct patch_entity *patch, const struct target_entity *target, unsigned long vma_start)
{
    void *file_buff = patch->file.buff;
    loff_t file_size = patch->file.size;

    ctx->target = &target->file;
    ctx->patch = &patch->file;
    ctx->load_bias = vma_start - target->file.vma_offset;
    log_debug("process %d: vma_start=0x%lx, load_bias=0x%lx\n", task_tgid_nr(current), vma_start, ctx->load_bias);

    // alloc & copy whole patch into kernel temporarily
    ctx->buff = vmalloc(file_size);
    if (!ctx->buff) {
        log_err("failed to vmalloc upatch info, len=0x%llx\n", file_size);
        return -ENOMEM;
    }
    memcpy(ctx->buff, file_buff, file_size);

    ctx->ehdr = ctx->buff;
    ctx->shdrs = ctx->buff + ctx->ehdr->e_shoff;

    ctx->shstrtab_shdr = &ctx->shdrs[patch->file.shstrtab_index];
    ctx->symtab_shdr = &ctx->shdrs[patch->file.symtab_index];
    ctx->strtab_shdr = &ctx->shdrs[patch->file.strtab_index];

    ctx->func_shdr = &ctx->shdrs[patch->file.func_index];
    ctx->rela_shdr = &ctx->shdrs[patch->file.rela_index];
    ctx->string_shdr = &ctx->shdrs[patch->file.string_index];

    // alloc patch sections
    layout_sections(ctx);

    // read process plt & got
    ctx->plt = vmalloc_copy_user((void __user *)ctx->load_bias, ctx->target->plt_addr, ctx->target->plt_size);
    if (IS_ERR(ctx->plt)) {
        ctx->plt = NULL;
    }

    ctx->got = vmalloc_copy_user((void __user *)ctx->load_bias, ctx->target->got_addr, ctx->target->got_size);
    if (IS_ERR(ctx->got)) {
        ctx->got = NULL;
    }

    return 0;
}

static void clear_load_info(struct patch_context *ctx)
{
    VFREE_CLEAR(ctx->buff);
    VFREE_CLEAR(ctx->plt);
    VFREE_CLEAR(ctx->got);
    KFREE_CLEAR(ctx->layout.kbase);
}

// we don't use the VMA after the patched code because we may use the VMA that heap will reuse
// we search before the patched code to find a empty VMA
static unsigned long find_vma_hole(unsigned long start, unsigned long size)
{
    struct mm_struct *mm = current->mm;

    unsigned long search = start;
    struct vm_area_struct *vma;

    mmap_read_lock(mm);
    vma = find_vma_intersection(mm, search, search + size);
    while (vma) {
        search = vma->vm_start - size;
        vma = find_vma_intersection(mm, search, search + size);
    }
    mmap_read_unlock(mm);

    log_debug("patch address: 0x%lx - 0x%lx\n", search, search + size);
    return search;
}

static int do_alloc_patch_memory(struct patch_context *ctx)
{
    /* find patch location & alloc patch from load bias */
    unsigned long base_addr = ctx->load_bias;
    unsigned long layout_size = ctx->layout.size; // must be page-aligned

    unsigned long vma_hole;
    unsigned long user_addr;
    unsigned long kern_addr;

    vma_hole = find_vma_hole(base_addr, layout_size);
    if (!vma_hole) {
        log_err("failed to find vma hole, addr=0x%lx, len=0x%lx\n", base_addr, layout_size);
        return -EFAULT;
    }

    user_addr = vm_mmap(NULL, vma_hole, layout_size, PROT_READ | PROT_WRITE, MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR_VALUE(user_addr)) {
        log_err("failed to patch memory in userspace\n");
        return -ENOMEM;
    }

    /* If the address applied for by the hot patch is too far away from the VMA address of the target file
     * and the relocation type (such as the jump instruction) with distance restriction is used during relocation
     * we can put the jump instruction into the jmp table to avoid the jump distance restriction
     */
    if (base_addr - user_addr >= PATCH_LOAD_RANGE_LIMIT) {
        log_warn("patch address range exceeds limit, addr=0x%lx, limit=0x%lx\n", user_addr, PATCH_LOAD_RANGE_LIMIT);
    }

    kern_addr = (unsigned long)kzalloc(layout_size, GFP_KERNEL);
    if (!kern_addr) {
        log_err("failed to alloc patch memory\n");
        vm_munmap(user_addr, layout_size);
        ctx->layout.base = 0;
        return -ENOMEM;
    }

    ctx->layout.kbase = (void *)kern_addr;
    ctx->layout.base = user_addr;
    return 0;
}

static void print_vma_info(void)
{
    struct mm_struct *mm = current->mm;
    struct upatch_vma_iter vma_iter;
    struct vm_area_struct *vma;
    char prot[5];
    char buff[256];
    const char *file_path;

    if (!mm) {
        log_debug("cannot find memory descriptor\n");
        return;
    }

    log_debug("virtual memory address:\n");
    mmap_read_lock(mm);
    {
        upatch_vma_iter_init(&vma_iter, mm);
        while ((vma = upatch_vma_next(&vma_iter))) {
            /* get file path or empty string */
            file_path = vma->vm_file ? d_path(&vma->vm_file->f_path, buff, sizeof(buff)) : "";
            /* parse protection flags */
            prot[0] = (vma->vm_flags & VM_READ)   ? 'r' : '-';
            prot[1] = (vma->vm_flags & VM_WRITE)  ? 'w' : '-';
            prot[2] = (vma->vm_flags & VM_EXEC)   ? 'x' : '-';
            prot[3] = (vma->vm_flags & VM_SHARED) ? 's' : 'p';
            prot[4] = '\0';
            /* print vma info */
            log_debug("0x%lx-0x%lx\t%s\t%s\n", vma->vm_start, vma->vm_end, prot, file_path);
        }
    }
    mmap_read_unlock(mm);
}

static int alloc_patch_memory(struct patch_context *ctx)
{
    struct patch_layout *layout = &ctx->layout;

    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Half shdr_num = ctx->ehdr->e_shnum;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab = ctx->buff + ctx->shstrtab_shdr->sh_offset;
    const char *sec_name;

    unsigned long dest;
    unsigned long kdest;

    /* Do the allocs. */
    int ret = do_alloc_patch_memory(ctx);
    if (ret) {
        return ret;
    }

    /* Transfer each section which specifies SHF_ALLOC */
    log_debug("final section addresses:\n");
    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        if (!(shdr->sh_flags & SHF_ALLOC)) {
            continue;
        }

        // sh_entsize is set to this section layout start offset in 'layout_sections'
        sec_name = shstrtab + shdr->sh_name; // no need check
        dest = layout->base + shdr->sh_entsize;
        kdest = (unsigned long)layout->kbase + shdr->sh_entsize;

        if (shdr->sh_type != SHT_NOBITS) {
            memcpy((void *)kdest, (void *)shdr->sh_addr, shdr->sh_size);
        }
        /* update sh_addr to point to copy in image. */
        shdr->sh_addr = (unsigned long)kdest;
        /* overuse this attr to record user address */
        shdr->sh_addralign = dest;
        log_debug("sec[%02d]  %-20s \t0x%lx -> 0x%lx size 0x%zx\n",
            i, sec_name, (unsigned long)kdest, dest, (size_t)shdr->sh_size);
    }

    log_debug("patch layout:\n");
    log_debug("\ttext          \t\t\t0x%lx size 0x%x\n", layout->base, layout->text_end);
    log_debug("\trodata        \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->text_end, layout->ro_end - layout->text_end);
    log_debug("\tro after init \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->ro_end, layout->ro_after_init_end - layout->ro_end);
    log_debug("\twritable      \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->ro_after_init_end, layout->size - layout->ro_after_init_end);

    print_vma_info();
    return 0;
}

static int simplify_symbols(struct patch_context *ctx)
{
    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Half shdr_num = ctx->ehdr->e_shnum;
    const char *shstrtab = (void *)ctx->shstrtab_shdr->sh_addr;

    Elf_Sym *symtab = (void *)ctx->symtab_shdr->sh_addr;
    size_t sym_num = ctx->symtab_shdr->sh_size / sizeof(Elf_Sym);

    const char *strtab = (void *)ctx->strtab_shdr->sh_addr;

    size_t i;
    Elf_Sym *sym;

    const char *sym_name;

    for (i = 1; i < sym_num; i++) {
        sym = &symtab[i];

        if (ELF_ST_TYPE(sym->st_info) == STT_SECTION && sym->st_shndx < shdr_num) {
            sym_name = shstrtab + shdrs[sym->st_shndx].sh_name;
        } else {
            sym_name = strtab + sym->st_name;
        }

        switch (sym->st_shndx) {
            case SHN_COMMON:
                log_err("common symbol '%s' is not supported\n", sym_name);
                return -ENOEXEC;
            case SHN_ABS:
                break;
            case SHN_UNDEF:
                sym->st_value = resolve_symbol(ctx, sym_name, sym);
                if (!sym->st_value) {
                    return -ENOEXEC;
                }
                log_debug("resolved external symbol '%s' at 0x%lx\n", sym_name, (unsigned long)sym->st_value);
                break;
            case SHN_LIVEPATCH:
                if (ctx->target->need_load_bias) {
                    sym->st_value += ctx->load_bias;
                }
                log_debug("resolved livepatch symbol '%s' at 0x%lx\n", sym_name, (unsigned long)sym->st_value);
                break;
            default:
                /* use userspace address */
                sym->st_value += shdrs[sym->st_shndx].sh_addralign;
                log_debug("resolved normal symbol '%s' -> 0x%lx\n", sym_name, (unsigned long)sym->st_value);
                break;
        }
    }

    return 0;
}

static int apply_relocations(struct patch_context *ctx)
{
    int ret;

    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Half shdr_num = ctx->ehdr->e_shnum;
    Elf_Half i;
    Elf_Shdr *shdr;

    const char *shstrtab = (void *)ctx->shstrtab_shdr->sh_addr;
    const char *sec_name;

    /* Now do relocations. */
    for (i = 1; i < shdr_num; i++) {
        shdr = &shdrs[i];

        if (shdr->sh_type != SHT_REL && shdr->sh_type != SHT_RELA) {
            continue;
        }

        /* not a valid relocation section? */
        if (shdr->sh_info >= shdr_num) {
            continue;
        }

        /* don't bother with non-allocated sections */
        if (!(shdrs[shdr->sh_info].sh_flags & SHF_ALLOC)) {
            continue;
        }

        sec_name = shstrtab + shdr->sh_name;
        log_debug("do relocations for %s\n", sec_name);
        ret = apply_relocate_add(ctx, i);
        if (ret) {
            return ret;
        }
    }

    return 0;
}

static int write_patch_to_user(const struct patch_context *ctx)
{
    const struct patch_layout *layout = &ctx->layout;

    log_debug("write patch image, dst=0x%lx, len=0x%x\n",
        layout->base, layout->size);
    if (copy_to_user((void *)layout->base, layout->kbase, layout->size)) {
        log_err("failed to write patch image, dst=0x%lx, len=0x%x\n",
        layout->base, layout->size);
        return -EFAULT;
    }

    return 0;
}

static int set_memory_privileges(const struct patch_context *ctx)
{
    const struct patch_layout *layout = &ctx->layout;

    unsigned long addr;
    size_t size;
    int ret;

    /* text */
    addr = layout->base;
    size = layout->text_end;
    ret = upatch_mprotect(addr, size, PROT_READ | PROT_EXEC);
    if (ret) {
        log_err("failed to set text memory privilege to r-x, ret=%d\n", ret);
        return ret;
    }

    /* rodata */
    addr = layout->base + layout->text_end;
    size = layout->ro_end - layout->text_end;
    ret = upatch_mprotect(addr, size, PROT_READ);
    if (ret) {
        log_err("failed to set rodata memory privilege to r--, ret=%d\n", ret);
        return ret;
    }

    /* ro_after_init */
    addr = layout->base + layout->ro_end;
    size = layout->ro_after_init_end - layout->ro_end;
    ret = upatch_mprotect(addr, size, PROT_READ);
    if (ret) {
        log_err("failed to set ro_after_init memory privilege to r--, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

/* The main idea is from insmod */
int upatch_resolve(struct target_entity *target, struct patch_entity *patch, struct process_entity *process,
    unsigned long vma_start)
{
    struct patch_context context = { 0 };
    int ret;

    ret = init_load_info(&context, patch, target, vma_start);
    if (ret) {
        goto fail;
    }

    ret = alloc_patch_memory(&context);
    if (ret) {
        goto fail;
    }

    /* Fix up syms, so that st_value is a pointer to location. */
    ret = simplify_symbols(&context);
    if (ret) {
        goto fail;
    }

    /* upatch new address will be updated */
    ret = apply_relocations(&context);
    if (ret) {
        goto fail;
    }

    ret = write_patch_to_user(&context);
    if (ret) {
        goto fail;
    }

    ret = set_memory_privileges(&context);
    if (ret) {
        goto fail;
    }

    ret = process_load_patch(process, patch, &context);
    if (ret) {
        goto fail;
    }

    log_debug("patch load successfully\n");
    clear_load_info(&context);
    return 0;

fail:
    if (context.layout.base) {
        vm_munmap(context.layout.base, context.layout.size);
        context.layout.base = 0;
    }
    clear_load_info(&context);
    return ret;
}

static inline bool is_addr_in_got(struct patch_context *ctx, Elf_Addr addr)
{
    unsigned long got_start = ctx->load_bias + ctx->target->got_addr;
    unsigned long got_end = got_start + ctx->target->got_size;

    unsigned long jmp_table_start = ctx->layout.base + ctx->layout.table.off;
    unsigned long jmp_table_end = jmp_table_start + ctx->layout.table.max;

    return (addr >= got_start && addr < got_end) || (addr >= jmp_table_start && addr < jmp_table_end);
}

unsigned long get_or_setup_got_entry(struct patch_context *ctx, Elf_Sym *sym)
{
    unsigned long got;

    if (sym->st_shndx == SHN_UNDEF && is_addr_in_got(ctx, sym->st_value)) {
        got = sym->st_value;
    } else {
        got = setup_got_table(ctx, sym->st_value, 0);
    }

    return got;
}