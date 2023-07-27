// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#ifdef __x86_64__

#include "arch/patch-load.h"

#ifndef R_X86_64_DTPMOD64
#define R_X86_64_DTPMOD64       16
#endif

#ifndef R_X86_64_TLSGD
#define R_X86_64_TLSGD          19
#endif

#ifndef R_X86_64_GOTTPOFF
#define R_X86_64_GOTTPOFF       22
#endif

#ifndef R_X86_64_TPOFF32
#define R_X86_64_TPOFF32        23
#endif

#ifndef R_X86_64_GOTPCRELX
#define R_X86_64_GOTPCRELX      41
#endif

#ifndef R_X86_64_REX_GOTPCRELX
#define R_X86_64_REX_GOTPCRELX  42
#endif

#define X86_64_JUMP_TABLE_JMP 0x90900000000225ff /* jmp [rip+2]; nop; nop */

void setup_parameters(struct pt_regs *regs, unsigned long para_a,
    unsigned long para_b, unsigned long para_c)
{
    regs->di = para_a;
    regs->si = para_b;
    regs->dx = para_c;
}

static unsigned long setup_jmp_table(struct upatch_load_info *info, unsigned long jmp_addr)
{
    struct upatch_jmp_table_entry *table = info->mod->core_layout.kbase + info->jmp_offs;
    unsigned int index = info->jmp_cur_entry;
    if (index >= info->jmp_max_entry) {
        pr_err("jmp table overflow \n");
        return 0;
    }

    table[index].inst = X86_64_JUMP_TABLE_JMP;
    table[index].addr = jmp_addr;
    info->jmp_cur_entry ++;
    return (unsigned long)(info->mod->core_layout.base + info->jmp_offs +
                           index * sizeof(struct upatch_jmp_table_entry));
}

/*
 * Jmp tabale records address and used call instruction to execute it.
 * So, we need 'Inst' and 'addr'
 * GOT only need record address and resolve it by [got_addr].
 * To simplify design, use same table for both jmp table and GOT.
 */
static unsigned long setup_got_table(struct upatch_load_info *info, unsigned long jmp_addr, unsigned long tls_addr)
{
    struct upatch_jmp_table_entry *table =
        info->mod->core_layout.kbase + info->jmp_offs;
    unsigned int index = info->jmp_cur_entry;
    if (index >= info->jmp_max_entry) {
        pr_err("got table overflow \n");
        return 0;
    }

    table[index].inst = jmp_addr;
    table[index].addr = tls_addr;
    info->jmp_cur_entry ++;
    return (unsigned long)(info->mod->core_layout.base + info->jmp_offs
        + index * sizeof(struct upatch_jmp_table_entry));
}

unsigned long insert_plt_table(struct upatch_load_info *info, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        pr_err("copy address failed \n");
        goto out;
    }

    elf_addr = setup_jmp_table(info, jmp_addr);

    pr_debug("0x%lx: jmp_addr=0x%lx \n", elf_addr, jmp_addr);

out:
    return elf_addr;
}


unsigned long insert_got_table(struct upatch_load_info *info, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        pr_err("copy address failed \n");
        goto out;
    }

    /*
     * R_X86_64_TLSGD: allocate two contiguous entries in the GOT to hold a tls_index structure
     * tls_index has two unsigned long, the first one is R_X86_64_DTPMOD64.
     */
    if (r_type == R_X86_64_DTPMOD64 &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        pr_err("copy address failed \n");
        goto out;
    }

    elf_addr = setup_got_table(info, jmp_addr, tls_addr);

    pr_debug("0x%lx: jmp_addr=0x%lx \n", elf_addr, jmp_addr);

out:
    return elf_addr;
}

int apply_relocate_add(struct upatch_load_info *info, Elf64_Shdr *sechdrs,
    const char *strtab, unsigned int symindex,
    unsigned int relsec, struct upatch_module *me)
{
    unsigned int i;
    Elf64_Rela *rel = (void *)sechdrs[relsec].sh_addr;
    Elf64_Sym *sym;
    void *loc, *real_loc;
    u64 val;
    const char *name;
    Elf64_Xword tls_size;

    pr_debug("Applying relocate section %u to %u\n",
             relsec, sechdrs[relsec].sh_info);

    for (i = 0; i < sechdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* This is where to make the change, calculate it from kernel address */
        loc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addr
            + rel[i].r_offset;

        real_loc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addralign
                 + rel[i].r_offset;

        /* This is the symbol it is referring to.  Note that all
           undefined symbols have been resolved. */
        sym = (Elf64_Sym *)sechdrs[symindex].sh_addr
            + ELF64_R_SYM(rel[i].r_info);
        name = strtab + sym->st_name;

        pr_debug("type %d st_value %Lx r_addend %Lx loc %Lx\n",
               (int)ELF64_R_TYPE(rel[i].r_info),
               sym->st_value, rel[i].r_addend, (u64)loc);

        val = sym->st_value + rel[i].r_addend;
        switch (ELF64_R_TYPE(rel[i].r_info)) {
        case R_X86_64_NONE:
            break;
        case R_X86_64_64:
            if (*(u64 *)loc != 0)
                goto invalid_relocation;
            memcpy(loc, &val, 8);
            break;
        case R_X86_64_32:
            if (*(u32 *)loc != 0)
                goto invalid_relocation;
            memcpy(loc, &val, 4);
            if (val != *(u32 *)loc
                && (ELF_ST_TYPE(sym->st_info) != STT_SECTION))
                goto overflow;
            break;
        case R_X86_64_32S:
            if (*(s32 *)loc != 0)
                goto invalid_relocation;
            memcpy(loc, &val, 4);
            if ((s64)val != *(s32 *)loc
                && (ELF_ST_TYPE(sym->st_info) != STT_SECTION))
                goto overflow;
            break;
        case R_X86_64_TLSGD:
        case R_X86_64_GOTTPOFF:
        case R_X86_64_GOTPCRELX:
        case R_X86_64_REX_GOTPCRELX:
            if (sym->st_value == 0)
                goto overflow;
            /* G + GOT + A*/
            val = sym->st_value + rel[i].r_addend;
            fallthrough;
        case R_X86_64_PC32:
        case R_X86_64_PLT32:
            if (*(u32 *)loc != 0)
                goto invalid_relocation;
            val -= (u64)real_loc;
            memcpy(loc, &val, 4);
            break;
        case R_X86_64_PC64:
            if (*(u64 *)loc != 0)
                goto invalid_relocation;
            val -= (u64)real_loc;
            memcpy(loc, &val, 8);
            break;
        case R_X86_64_TPOFF32:
            tls_size = ALIGN(info->running_elf.tls_size, info->running_elf.tls_align);
            // %fs + val - tls_size
            if (val >= tls_size)
                goto overflow;
            val -= (u64)tls_size;
            memcpy(loc, &val, 4);
            break;
        default:
            pr_err("Unknown rela relocation: %llu\n", ELF64_R_TYPE(rel[i].r_info));
            return -ENOEXEC;
        }
    }
    return 0;

invalid_relocation:
    pr_err("upatch: Skipping invalid relocation target, \
        existing value is nonzero for type %d, loc %p, name %s\n",
        (int)ELF64_R_TYPE(rel[i].r_info), loc, name);
    return -ENOEXEC;

overflow:
    pr_err("upatch: overflow in relocation type %d name %s\n",
           (int)ELF64_R_TYPE(rel[i].r_info), name);
    return -ENOEXEC;
}

#endif /* __x86_64__ */
