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

#include "patch_entity.h"
#include "target_entity.h"
#include "process_entity.h"
#include "symbol_resolve.h"
#include "kernel_compat.h"
#include "util.h"

#define FLAG_LEN       3

#ifndef ARCH_SHF_SMALL
#define ARCH_SHF_SMALL 0
#endif

static int setup_load_info(struct upatch_info *info, struct patch_entity *patch)
{
    info->len = patch->meta.file_size;
    info->ehdr = vmalloc(info->len);
    if (!info->ehdr) {
        log_err("failed to vmalloc upatch info, len=%ld\n", info->len);
        return -ENOMEM;
    }

    // read whole patch into kernel temporarily
    memcpy(info->ehdr, patch->meta.file_buff, info->len);

    info->shdrs = (void *)info->ehdr + info->ehdr->e_shoff;
    info->shshdrtab = (void *)info->ehdr + info->shdrs[info->ehdr->e_shstrndx].sh_offset;
    info->index.sym = patch->meta.symtab_index;
    info->index.str = patch->meta.strtab_index;
    info->und_cnt = patch->meta.und_sym_num;
    info->got_rela_cnt = patch->meta.got_reloc_num;
    info->strtab = (char *)info->ehdr + info->shdrs[info->index.str].sh_offset;
    log_debug("symbol '%d', type UND, got_rela=%d\n", info->und_cnt, info->got_rela_cnt);

    return 0;
}

static int rewrite_section_headers(struct upatch_info *info)
{
    unsigned int i;

    /* This should always be true, but let's be sure. */
    info->shdrs[0].sh_addr = 0;
    info->shdrs[0].sh_addralign = 0;

    for (i = 1; i < info->ehdr->e_shnum; i++) {
        Elf_Shdr *shdr = &info->shdrs[i];
        if (shdr->sh_type != SHT_NOBITS && info->len < shdr->sh_offset + shdr->sh_size) {
            log_err("section was truncated, index=%u, len=%lu\n", i, info->len);
            return -ENOEXEC;
        }

        /* Mark all sections sh_addr with their address in the
           temporary image. */
        shdr->sh_addr = (size_t)info->ehdr + shdr->sh_offset;
    }

    return 0;
}

static long align_size_add_sh_size(unsigned int *size, Elf_Shdr *sechdr)
{
    long ret;

    ret = ALIGN(*size, sechdr->sh_addralign ?: 1);
    *size = ret + sechdr->sh_size;
    return ret;
}

static void layout_jmptable(struct upatch_layout *layout, struct upatch_info *info)
{
    unsigned long start_off;
    layout->table.cur = 0;
    layout->table.max = info->und_cnt * JMP_TABLE_ENTRY_MAX_SIZE + info->got_rela_cnt * JMP_TABLE_GOT_ENTRY_SIZE;
    start_off = ALIGN(layout->size, sizeof(unsigned long));

    layout->table.off = start_off;
    layout->size = start_off + layout->table.max;
    log_debug("\t\t%-20s \t0x%lx - 0x%x max size %d\n", "jmptable", start_off, layout->size, layout->table.max);
}

static void layout_sections(struct upatch_layout *layout, struct upatch_info *info)
{
    static unsigned long const masks[][2] = {
        /* NOTE: all executable code must be the first section
         * in this array; otherwise modify the text_size
         * finder in the two loops below */
        { SHF_EXECINSTR | SHF_ALLOC,    ARCH_SHF_SMALL },
        { SHF_ALLOC,            SHF_WRITE | ARCH_SHF_SMALL },
        { SHF_RO_AFTER_INIT | SHF_ALLOC, ARCH_SHF_SMALL },
        { SHF_WRITE | SHF_ALLOC,    ARCH_SHF_SMALL },
        { ARCH_SHF_SMALL | SHF_ALLOC,   0 }
    };
    unsigned int m;
    unsigned int i;

    for (i = 0; i < info->ehdr->e_shnum; i++) {
        info->shdrs[i].sh_entsize = ~0UL;
    }

    log_debug("upatch section allocation order:\n");
    for (m = 0; m < ARRAY_SIZE(masks); ++m) {
        for (i = 0; i < info->ehdr->e_shnum; ++i) {
            Elf_Shdr *s = &info->shdrs[i];
            const char *sname = info->shshdrtab + s->sh_name;

            if ((s->sh_flags & masks[m][0]) != masks[m][0] || (s->sh_flags & masks[m][1]) || s->sh_entsize != ~0UL) {
                continue;
            }
            s->sh_entsize = align_size_add_sh_size(&layout->size, s);
            log_debug("type[%02d] %-20s \tend at 0x%x\n", m, sname, layout->size);
        }
        switch (m) {
            case 0: /* executable */
                layout_jmptable(layout, info);
                layout->size = PAGE_ALIGN(layout->size);
                layout->text_end = layout->size;
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

// we don't use the VMA after the patched code because we may use the VMA that heap will reuse
// we search before the patched code to find a empty VMA
static unsigned long get_upatch_hole(unsigned long start, unsigned long size)
{
    unsigned long search = start;
    struct vm_area_struct *vma;
    struct mm_struct *mm = current->mm;

    mmap_read_lock(mm);
    vma = find_vma_intersection(mm, search, search + size);
    while (vma) {
        search = vma->vm_start - size;
        vma = find_vma_intersection(mm, search, search + size);
    }
    mmap_read_unlock(mm);

    log_debug("find hole at 0x%lx - 0x%lx\n", search, search + size);
    return search;
}

/* alloc memory in userspace */
static unsigned long find_vma_hole_and_vmmap(unsigned long vma_start, unsigned long size)
{
    unsigned long mem_addr;
    unsigned long addr = get_upatch_hole(vma_start, size);
    if (!addr) {
        log_err("cannot find hole start in %ld in pid %d\n", vma_start, task_pid_nr(current));
        return 0;
    }

    mem_addr = vm_mmap(NULL, addr, size, PROT_READ | PROT_WRITE,
        MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)mem_addr)) {
        log_err("mmap process memory failed with %ld\n", PTR_ERR((void *)mem_addr));
        return 0;
    }

    /* If the address applied for by the hot patch is too far away from the VMA address of the target file
     * and the relocation type (such as the jump instruction) with distance restriction is used during relocation
     * we can put the jump instruction into the jmp table to avoid the jump distance restriction
     */
    if (vma_start - mem_addr >= PATCH_LOAD_RANGE_LIMIT) {
        log_warn("patch hole 0x%lx is 0x%lx far from the code start 0x%lx\n",
            mem_addr, vma_start - mem_addr, vma_start);
    }

    return mem_addr;
}

static int alloc_mem_for_upatch(struct upatch_info *info, struct upatch_layout *layout)
{
    /* find moudle location from code start place */
    unsigned long vma_start = info->running_elf.vma_start_addr;

    layout->base = find_vma_hole_and_vmmap(vma_start, layout->size);
    if (!layout->base)
        return -ENOMEM;

    layout->kbase = kzalloc(layout->size, GFP_KERNEL);
    if (!layout->kbase) {
        vm_munmap(layout->base, layout->size);
        layout->base = 0;
        return -ENOMEM;
    }

    log_debug("kbase 0x%lx base 0x%lx\n", (unsigned long)(uintptr_t)layout->kbase, layout->base);

    return 0;
}

static void clear_load_info(struct upatch_info *info)
{
    VFREE_CLEAR(info->ehdr);
    KFREE_CLEAR(info->layout.kbase);
}

void parse_vma_flags(char *buf, unsigned long flags)
{
    buf[0] = (flags & VM_READ)  ? 'r' : '-';
    buf[1] = (flags & VM_WRITE) ? 'w' : '-';
    buf[2] = (flags & VM_EXEC)  ? 'x' : '-';
    buf[3] = '\0';
}

static void print_vma_info(void)
{
    struct mm_struct *mm = current->mm;
    struct vm_area_struct *vma;
    struct upatch_vma_iter vma_iter;
    char prot[4];
    char *path;
    char buf[256];

    if (!mm) {
        log_debug("cannot find memory descriptor\n");
        return;
    }

    log_debug("virtual memory address:\n");
    mmap_read_lock(mm);
    upatch_vma_iter_init(&vma_iter, mm);
    while ((vma = upatch_vma_next(&vma_iter))) {
        if (vma->vm_file) {
            path = d_path(&vma->vm_file->f_path, buf, sizeof(buf));
        } else {
            path = "";
        }
        parse_vma_flags(prot, vma->vm_flags);
        log_debug("0x%lx-0x%lx %s %s\n", vma->vm_start, vma->vm_end, prot, path);
    }
    mmap_read_unlock(mm);
}

static int alloc_layout(struct upatch_layout *layout, struct upatch_info *info)
{
    char *name;
    int i;
    int ret;

    /* Do the allocs. */
    ret = alloc_mem_for_upatch(info, layout);
    if (ret) {
        log_err("failed to alloc upatch process memory, ret=%d\n", ret);
        return ret;
    }

    /* Transfer each section which specifies SHF_ALLOC */
    log_debug("final section addresses:\n");
    for (i = 0; i < info->ehdr->e_shnum; i++) {
        unsigned long dest;
        uintptr_t kdest;
        Elf_Shdr *shdr = &info->shdrs[i];

        if (!(shdr->sh_flags & SHF_ALLOC)) {
            continue;
        }

        name = info->shshdrtab + shdr->sh_name;

        // sh_entsize is set to this section layout start offset in 'layout_sections'
        dest = layout->base + shdr->sh_entsize;
        kdest = (uintptr_t)layout->kbase + shdr->sh_entsize;

        if (shdr->sh_type != SHT_NOBITS)
            memcpy((void *)kdest, (void *)shdr->sh_addr, shdr->sh_size);

        if (!strcmp(name, ".upatch.funcs")) {
            info->upatch_func_sec = shdr;
        }

        /* Update sh_addr to point to copy in image. */
        shdr->sh_addr = (unsigned long)kdest;
        /* overuse this attr to record user address */
        shdr->sh_addralign = dest;
        log_debug("sec[%02d]  %-20s \t0x%lx -> 0x%lx size 0x%zx\n",
            i, name, (unsigned long)kdest, dest, (size_t)shdr->sh_size);
    }

    log_debug("patch vma layout:\n");
    log_debug("\ttext          \t\t\t0x%lx size 0x%x\n", layout->base, layout->text_end);
    log_debug("\trodata        \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->text_end, layout->ro_end - layout->text_end);
    log_debug("\tro after init \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->ro_end, layout->ro_after_init_end - layout->ro_end);
    log_debug("\twritable      \t\t\t0x%lx size 0x%x\n",
        layout->base + layout->ro_after_init_end, layout->size - layout->ro_after_init_end);

    print_vma_info();

    if (!info->upatch_func_sec) {
        log_err("cannot find '.upatch_func' section\n");
        return -1;
    }
    return 0;
}

static int layout_and_allocate(struct upatch_info *info)
{
    int err;

    layout_sections(&info->layout, info);

    err = alloc_layout(&info->layout, info);
    if (err) {
        return err;
    }

    return 0;
}

static int simplify_symbols(const struct upatch_info *info)
{
    Elf_Shdr *symsec = &info->shdrs[info->index.sym];
    Elf_Sym *sym = (void *)symsec->sh_addr;
    unsigned long secbase;
    unsigned int i;
    int ret = 0;
    unsigned long elf_addr;

    for (i = 1; i < symsec->sh_size / sizeof(Elf_Sym); i++) {
        const char *name;

        if (ELF_ST_TYPE(sym[i].st_info) == STT_SECTION && sym[i].st_shndx < info->ehdr->e_shnum) {
            name = info->shshdrtab + info->shdrs[sym[i].st_shndx].sh_name;
        } else {
            name = info->strtab + sym[i].st_name;
        }

        switch (sym[i].st_shndx) {
            case SHN_COMMON:
                log_err("common symbol '%s' is not supported\n", name);
                ret = -ENOEXEC;
                break;
            case SHN_ABS:
                break;
            case SHN_UNDEF:
                elf_addr = resolve_symbol(&info->running_elf, name, sym[i]);
                if (!elf_addr) {
                    return -ENOEXEC;
                }
                sym[i].st_value = elf_addr;
                log_debug("resolved external symbol '%s' at 0x%lx\n",
                    name, (unsigned long)sym[i].st_value);
                break;
            case SHN_LIVEPATCH:
                sym[i].st_value += info->running_elf.vma_start_addr;
                log_debug("resolved livepatch symbol '%s' at 0x%lx\n",
                    name, (unsigned long)sym[i].st_value);
                break;
            default:
                /* use real address to calculate secbase */
                secbase = info->shdrs[sym[i].st_shndx].sh_addralign;
                sym[i].st_value += secbase;
                log_debug("resolved normal symbol '%s' -> 0x%lx\n",
                    name, (unsigned long)sym[i].st_value);
                break;
        }
    }

    return ret;
}

static int apply_relocations(struct upatch_info *info)
{
    unsigned int i;
    int err = 0;

    /* Now do relocations. */
    for (i = 1; i < info->ehdr->e_shnum; i++) {
        unsigned int infosec = info->shdrs[i].sh_info;
        const char *name = info->shshdrtab + info->shdrs[i].sh_name;

        /* Not a valid relocation section? */
        if (infosec >= info->ehdr->e_shnum) {
            continue;
        }

        /* Don't bother with non-allocated sections */
        if (!(info->shdrs[infosec].sh_flags & SHF_ALLOC)) {
            continue;
        }

        if (info->shdrs[i].sh_type == SHT_REL || info->shdrs[i].sh_type == SHT_RELA) {
            log_debug("do relocations for %s\n", name);
            err = apply_relocate_add(info, i);
        }

        if (err) {
            break;
        }
    }
    return err;
}

static int copy_layout_into_vma(struct upatch_layout *layout)
{
    log_debug("mov content from 0x%lx to 0x%lx with 0x%x\n",
        (unsigned long)layout->kbase, layout->base, layout->size);
    if (copy_to_user((void *)layout->base, layout->kbase, layout->size)) {
        log_err("copy_to_user from 0x%lx to 0x%lx with 0x%x failed\n",
            (unsigned long)layout->kbase, layout->base, layout->size);
        return -EPERM;
    }
    return 0;
}

static int frob_text(const struct upatch_layout *layout)
{
    unsigned long addr = (unsigned long)layout->base;
    size_t text_size = layout->text_end;
    int ret;

    ret = upatch_mprotect(addr, text_size, PROT_READ | PROT_EXEC);
    if (ret) {
        log_err("failed to set text memory previliage to r-x, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

static int frob_rodata(const struct upatch_layout *layout)
{
    unsigned long ro_start = (unsigned long)layout->base + layout->text_end;
    size_t ro_size = layout->ro_end - layout->text_end;
    int ret;

    unsigned long ro_after_init_start = (unsigned long)layout->base + layout->ro_end;
    size_t ro_after_init_size = layout->ro_after_init_end - layout->ro_end;

    ret = upatch_mprotect(ro_start, ro_size, PROT_READ);
    if (ret) {
        log_err("failed to set rodata memory previliage to r--, ret=%d\n", ret);
        return ret;
    }

    ret = upatch_mprotect(ro_after_init_start, ro_after_init_size, PROT_READ);
    if (ret) {
        log_err("failed to set ro_after_init memory previliage to r--, ret=%d\n", ret);
        return ret;
    }

    return 0;
}

static int set_memory_previliage(struct upatch_layout *layout)
{
    int ret;

    ret = frob_text(layout);
    if (ret) {
        return ret;
    }

    ret = frob_rodata(layout);
    if (ret) {
        return ret;
    }

    return 0;
}

// create old_pc - new_pc maps
static int create_relocated_pc_maps(struct process_entity *process, struct upatch_info *load_info,
    struct patch_entity *patch)
{
    unsigned int num;
    struct upatch_function *funcs;
    unsigned int i;
    struct patch_info *info;
    struct pc_pair *pp;

    funcs = (struct upatch_function *)load_info->upatch_func_sec->sh_addr;
    num = load_info->upatch_func_sec->sh_size / (sizeof (struct upatch_function));

    info = kzalloc(sizeof(struct patch_info), GFP_KERNEL);
    if (!info) {
        log_err("malloc patch_info failed!\n");
        return -ENOMEM;
    }

    hash_init(info->pc_maps);
    for (i = 0; i < num; ++i) {
        pp = kmalloc(sizeof(*pp), GFP_KERNEL);
        if (!pp) {
            free_patch_info(info);
            return -ENOMEM;
        }
        pp->old_pc = funcs[i].old_addr +
            load_info->running_elf.vma_start_addr +
            load_info->running_elf.meta->load_offset;
        pp->new_pc = funcs[i].new_addr;
        hash_add(info->pc_maps, &pp->node, pp->old_pc);
        log_debug("function: 0x%08lx -> 0x%08lx\n", pp->old_pc, pp->new_pc);
    }

    list_add(&info->list, &process->loaded_patches);
    info->patch = patch;
    process->active_info = info;

    return 0;
}

/* The main idea is from insmod */
int upatch_resolve(struct target_entity *target, struct patch_entity *patch, struct process_entity *process,
    unsigned long target_code_start)
{
    struct upatch_info info;
    int err;

    memset(&info, 0, sizeof(info));

    info.running_elf.vma_start_addr = target_code_start - target->meta.vma_offset;
    log_debug("process %d: vma_start=0x%lx, code_start=0x%lx\n",
        task_pid_nr(current), info.running_elf.vma_start_addr, target_code_start);

    info.running_elf.meta = &target->meta;
    info.running_elf.load_info = &info;

    err = setup_load_info(&info, patch);
    if (err) {
        goto fail;
    }

    /* update section address */
    err = rewrite_section_headers(&info);
    if (err) {
        goto fail;
    }

    err = layout_and_allocate(&info);
    if (err) {
        goto fail;
    }

    /* Fix up syms, so that st_value is a pointer to location. */
    err = simplify_symbols(&info);
    if (err) {
        goto fail;
    }

    /* upatch new address will be updated */
    err = apply_relocations(&info);
    if (err) {
        goto fail;
    }

    err = copy_layout_into_vma(&info.layout);
    if (err) {
        goto fail;
    }

    err = set_memory_previliage(&info.layout);
    if (err) {
        goto fail;
    }

    err = create_relocated_pc_maps(process, &info, patch);
    if (err) {
        goto fail;
    }

    log_debug("patch load successfully\n");
    clear_load_info(&info);
    return 0;

fail:
    if (info.layout.base) {
        vm_munmap(info.layout.base, info.layout.size);
        info.layout.base = 0;
    }
    clear_load_info(&info);
    return err;
}

static inline bool is_addr_in_got_table(struct upatch_layout *layout, u64 addr)
{
    unsigned long table_start = layout->base + layout->table.off;
    unsigned long table_end = table_start + layout->table.max;
    return addr >= table_start && addr < table_end;
}

unsigned long get_or_setup_got_entry(struct upatch_info *info, Elf_Sym *sym)
{
    unsigned long got;

    if (sym->st_shndx == SHN_UNDEF && is_addr_in_got_table(&info->layout, sym->st_value)) {
        got = sym->st_value;
    } else {
        got = setup_got_table(info, sym->st_value, 0);
    }

    return got;
}