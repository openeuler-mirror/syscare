// SPDX-License-Identifier: GPL-2.0
/*
 * setup jmp table and do relocation in x86
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

#ifdef __x86_64__

#include <linux/uaccess.h>

#include "../patch_load.h"

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

#define X86_64_JUMP_TO_FUNC 0x90900000000225ff  // jmp [rip+2]; nop; nop

/* For IFUNC(indirect function), the symbol value is point to the resolve function
 * We should call resolve func and get the real func address in rax
 * In x86_64 function calling convention, we should save all reg that is already saved args.
 * For all IFUNC type func in glibc, it will use at most 3 args, so we only save rdi, rsi, rdx
 * 0:   push    rdi
 * 1:   push    rsi
 * 2:   push    rdx
 * 3:   call    QWORD PTR [rip+0x7]
 * 9:   pop     rdx
 * a:   pop     rsi
 * b:   pop     rdi
 * c:   jmp     rax
 * e:   nop
 * f:   nop
 * 10-18: <address>
 */

#define X86_64_CALL_IFUNC_1 0x00000715FF525657
#define X86_64_CALL_IFUNC_2 0x9090E0FF5F5E5A00

unsigned long setup_jmp_table(struct patch_context *ctx, unsigned long jmp_addr, bool is_ifunc)
{
    struct jmp_table *table = &ctx->layout.table;
    unsigned long *jmp = ctx->layout.kbase + table->off;
    unsigned int index = table->cur;
    int entry_num = is_ifunc ? IFUNC_JMP_ENTRY_NUM : NORMAL_JMP_ENTRY_NUM;
    if (table->cur + entry_num > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, entry_num);
        return 0;
    }

    if (is_ifunc) {
        jmp[index]      = X86_64_CALL_IFUNC_1;
        jmp[index + 1]  = X86_64_CALL_IFUNC_2;
        jmp[index + 2]  = jmp_addr;
    } else {
        jmp[index]      = X86_64_JUMP_TO_FUNC;
        jmp[index + 1]  = jmp_addr;
    }
    table->cur += entry_num;

    return ctx->layout.base + table->off + index * JMP_ENTRY_SIZE;
}

/*
 * Jmp table records address and used call instruction to execute it.
 * So, we need 'Inst' and 'addr'
 * GOT only need record address and resolve it by [got_addr].
 * To simplify design, use same table for both jmp table and GOT.
 */
unsigned long setup_got_table(struct patch_context *ctx, unsigned long jmp_addr, unsigned long tls_addr)
{
    struct jmp_table *table = &ctx->layout.table;
    unsigned long *jmp = ctx->layout.kbase + table->off;
    unsigned int index = table->cur;
    unsigned long entry_addr = ctx->layout.base + table->off + index * JMP_ENTRY_SIZE;
    if (table->cur + NORMAL_JMP_ENTRY_NUM > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, NORMAL_JMP_ENTRY_NUM);
        return 0;
    }

    jmp[index] = jmp_addr;
    jmp[index + 1] = tls_addr;
    table->cur += NORMAL_JMP_ENTRY_NUM;

    log_debug("\tsetup got table at 0x%lx -> 0x%lx, tls_addr = 0x%lx\n",
        entry_addr, jmp_addr, tls_addr);

    return entry_addr;
}

unsigned long insert_plt_table(struct patch_context *ctx, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    elf_addr = setup_jmp_table(ctx, jmp_addr, false);

    log_debug("PLT: 0x%lx -> 0x%lx\n", elf_addr, jmp_addr);

out:
    return elf_addr;
}


unsigned long insert_got_table(struct patch_context *ctx, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    /*
     * R_X86_64_TLSGD: allocate two contiguous entries in the GOT to hold a tls_index structure
     * tls_index has two unsigned long, the first one is R_X86_64_DTPMOD64.
     */
    if (r_type == R_X86_64_DTPMOD64 &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    elf_addr = setup_got_table(ctx, jmp_addr, tls_addr);

out:
    return elf_addr;
}

int apply_relocate_add(struct patch_context *ctx, unsigned int relsec)
{
    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Sym *symtab = (void *)ctx->symtab_shdr->sh_addr;
    const char *strtab = (void *)ctx->strtab_shdr->sh_addr;
    unsigned int i;
    Elf_Rela *rel = (void *)shdrs[relsec].sh_addr;
    Elf_Sym *sym;
    void *reloc_place;
    void *ureloc_place;
    u64 sym_addr;
    u64 got;
    const char *name;
    Elf_Addr tls_size;

    unsigned int reloc_sec = shdrs[relsec].sh_info;

    // sh_addr = kdest, is the section start in hot patch kalloc memory
    // sh_addralign = dest, is the section start in VMA pole
    void *sec_kaddr = (void *)shdrs[reloc_sec].sh_addr;
    void *sec_vaddr = (void *)shdrs[reloc_sec].sh_addralign;

    log_debug("Applying relocate section %u to %u\n", relsec, reloc_sec);
    log_debug("section %d: kernel address = 0x%llx, virtual address = 0x%llx\n",
        reloc_sec, (u64)sec_kaddr, (u64)sec_vaddr);

    for (i = 0; i < shdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* This is where to make the change, calculate it from kernel address. */

        /* corresponds P in the kernel space */
        reloc_place = sec_kaddr + rel[i].r_offset;

        /* corresponds P in user space */
        ureloc_place = sec_vaddr + rel[i].r_offset;

        /* This is the symbol it is referring to. Note that all
            undefined symbols have been resolved. */
        sym = &symtab[ELF_R_SYM(rel[i].r_info)];
        name = strtab + sym->st_name;

        /* src corresponds to (S + A) */
        sym_addr = sym->st_value + rel[i].r_addend;

        log_debug("'%s'\t type %d st_value 0x%llx r_addend %ld r_offset 0x%llx\n",
            name, (int)ELF_R_TYPE(rel[i].r_info), sym->st_value, (long int)rel[i].r_addend, rel[i].r_offset);
        log_debug("\t(S + A) = 0x%llx \tP(kernel) = 0x%Lx \tP(user) = 0x%Lx\n",
            sym_addr, (u64)reloc_place, (u64)ureloc_place);
        log_debug("\t(before) *reloc_place = 0x%llx\n", *(u64*)reloc_place);
        switch (ELF_R_TYPE(rel[i].r_info)) {
            case R_X86_64_NONE:
                break;
            case R_X86_64_64:
                memcpy(reloc_place, &sym_addr, sizeof(u64));
                break;
            case R_X86_64_32:
                memcpy(reloc_place, &sym_addr, sizeof(u32));
                if (sym_addr != *(u32 *)reloc_place
                    && (ELF_ST_TYPE(sym->st_info) != STT_SECTION)) {
                    goto overflow;
                }
                break;
            case R_X86_64_32S:
                memcpy(reloc_place, &sym_addr, sizeof(u32));
                if ((s64)sym_addr != *(s32 *)reloc_place && (ELF_ST_TYPE(sym->st_info) != STT_SECTION)) {
                    goto overflow;
                }
                break;
            case R_X86_64_TLSGD:
            case R_X86_64_GOTTPOFF:
            case R_X86_64_GOTPCRELX:
            case R_X86_64_REX_GOTPCRELX:
                /* get GOT address */
                got = get_or_setup_got_entry(ctx, sym);
                if (got == 0) {
                    goto overflow;
                }
                // G + GOT + A
                sym_addr = got + rel[i].r_addend;
                // G + GOT + A - P
                fallthrough;
            case R_X86_64_PC32:
            case R_X86_64_PLT32:
                sym_addr -= (u64)ureloc_place;
                memcpy(reloc_place, &sym_addr, sizeof(u32));
                break;
            case R_X86_64_PC64:
                sym_addr -= (u64)ureloc_place;
                memcpy(reloc_place, &sym_addr, sizeof(u64));
                break;
            case R_X86_64_TPOFF32:
                tls_size = ALIGN(ctx->target->tls_size, ctx->target->tls_align);
                // %fs + val - tls_size
                if (sym_addr >= tls_size) {
                    goto overflow;
                }
                sym_addr -= (u64)tls_size;
                memcpy(reloc_place, &sym_addr, sizeof(u32));
                break;
            default:
                log_err("\tUnknown rela relocation: %llu\n", ELF_R_TYPE(rel[i].r_info));
                return -ENOEXEC;
        }
        log_debug("\t(after) *reloc_place = 0x%llx\n", *(u64*)reloc_place);
    }
    return 0;

overflow:
    log_err("\toverflow in relocation type %d name %s\n",
        (int)ELF_R_TYPE(rel[i].r_info), name);
    return -ENOEXEC;
}

bool is_got_rela_type(int type)
{
    switch (type) {
        case R_X86_64_TLSGD:
        case R_X86_64_GOTTPOFF:
        case R_X86_64_GOTPCRELX:
        case R_X86_64_REX_GOTPCRELX:
            return true;
        default:
            break;
    }
    return false;
}

#endif /* __x86_64__ */
