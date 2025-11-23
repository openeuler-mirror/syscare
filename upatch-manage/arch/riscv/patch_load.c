// SPDX-License-Identifier: GPL-2.0
/*
 * setup jmp table and do relocation in riscv64
 * Copyright (C) 2024 ISCAS
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

#ifdef __riscv // now only for riscv64

#include <linux/uaccess.h>

#include "../../util.h"
#include "../patch_load.h"
#include "insn.h"

/* For IFUNC(indirect function), the symbol value is point to the resolve function
 * We should call resolve func and get the real func address in a0
 * In riscv64 calling convention, we should save all arg registers (a0, a1, a2)
 * and the return address (ra) before calling resolver.
 * After calling, we restore the registers and jump to the real function addr.
 *
 * 0x00: addi  sp, sp, -32
 * 0x04: sd    a0, 0(sp)
 * 0x08: sd    a1, 8(sp)
 * 0x0c: sd    a2, 16(sp)
 * 0x10: sd    ra, 24(sp)
 * 0x14: auipc t0, 0
 * 0x18: ld    t0, 32(t0)
 * 0x1c: jalr  t0
 * 0x20: ld    ra, 24(sp)
 * 0x24: ld    a2, 16(sp)
 * 0x28: ld    a1, 8(sp)
 * 0x2c: ld    a0, 0(sp)
 * 0x30: addi  sp, sp, 32
 * 0x34: jr    a0
 * 0x38: <resolver address>  // 8-byte address
 */
#define RISCV64_CALL_IFUNC_1 0x000280e7202b0297ULL  // jalr t0; ld t0, 32(t0); auipc t0, 0
#define RISCV64_CALL_IFUNC_2 0x0201011300358503ULL  // addi sp, 32; ld a0, 0(sp)
#define RISCV64_CALL_IFUNC_3 0x0081358301033603ULL  // ld a1, 8(sp); ld a2, 16(sp)
#define RISCV64_CALL_IFUNC_4 0x0181308300000513ULL  // ld ra, 24(sp); nop (addi a0, zero, 0)
#define RISCV64_CALL_IFUNC_5 0x01d1382300c13423ULL  // sd ra, 24(sp); sd a2, 16(sp)
#define RISCV64_CALL_IFUNC_6 0x00b1302300010113ULL  // sd a1, 8(sp); addi sp, -32
#define RISCV64_CALL_IFUNC_7 0x0000000000000023ULL  // sd a0, 0(sp); padding


/*
 * auipc   t6,0x0
 * ld      t6,16(t6) # addr
 * jr      t6
 * undefined
 */
#define RISCV64_JMP_TABLE_JUMP0 0x010fbf8300000f97
#define RISCV64_JMP_TABLE_JUMP1 0x00000000000f8067

#ifndef R_RISCV_RVC_LUI
#define R_RISCV_RVC_LUI          46
#endif

#ifndef R_RISCV_PLT32
#define R_RISCV_PLT32            59
#endif


/*
 * In PCREL_LO12 relocation entity, its corresponding symbol's value
 * points to the ..._HI20 instruction, where the LO12 part of the
 * immediate is part of the ..._HI20 symbol value.
 */
static unsigned long find_pcrel_hi_value(Elf_Rela *r, int idx, Elf_Sym *st, unsigned long v)
{
    int i = idx;
    r--;
    for (; i > 0; i--, r--) {
        if ((r->r_offset == v) &&
            ((ELF_R_TYPE(r->r_info) == R_RISCV_PCREL_HI20) ||
             (ELF_R_TYPE(r->r_info) == R_RISCV_TLS_GOT_HI20) ||
             (ELF_R_TYPE(r->r_info) == R_RISCV_TLS_GD_HI20) ||
             (ELF_R_TYPE(r->r_info) == R_RISCV_GOT_HI20)))
            return st[ELF_R_SYM(r->r_info)].st_value;
    }

    /* should never happen */
    log_err("Not found no. %d rela's corresponding HI20\n", idx);
    return 0;
}

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
        jmp[index]      = RISCV64_CALL_IFUNC_1;
        jmp[index + 1]  = RISCV64_CALL_IFUNC_2;
        jmp[index + 2]  = RISCV64_CALL_IFUNC_3;
        jmp[index + 3]  = RISCV64_CALL_IFUNC_4;
        jmp[index + 4]  = RISCV64_CALL_IFUNC_5;
        jmp[index + 5]  = RISCV64_CALL_IFUNC_6;
        jmp[index + 6]  = RISCV64_CALL_IFUNC_7;
        jmp[index + 7]  = jmp_addr;
    } else {
        jmp[index] = RISCV64_JMP_TABLE_JUMP0;
        jmp[index + 1] = RISCV64_JMP_TABLE_JUMP1;
        jmp[index + 2] = jmp_addr;
    }
    table->cur += entry_num;

    return ctx->layout.base + table->off + index * JMP_ENTRY_SIZE;
}

static unsigned long setup_jmp_table_with_plt(struct patch_context *ctx,
    unsigned long jmp_addr, unsigned long plt_addr)
{
    struct jmp_table *table = &ctx->layout.table;
    unsigned long *jmp = ctx->layout.kbase + table->off;
    unsigned int index = table->cur;
    int entry_num = PLT_JMP_ENTRY_NUM;
    if (table->cur + entry_num > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, entry_num);
        return 0;
    }

    jmp[index] = RISCV64_JMP_TABLE_JUMP0;
    jmp[index + 1]  = RISCV64_JMP_TABLE_JUMP1;
    jmp[index + 2]  = jmp_addr;
    jmp[index + 3]  = plt_addr;
    table->cur += entry_num;

    return ctx->layout.base + table->off + index * JMP_ENTRY_SIZE;
}

unsigned long setup_got_table(struct patch_context *ctx, unsigned long jmp_addr, unsigned long tls_addr)
{
    struct jmp_table *table = &ctx->layout.table;
    unsigned long *jmp = ctx->layout.kbase + table->off;
    unsigned int index = table->cur;
    unsigned long entry_addr = ctx->layout.base + table->off + index * JMP_ENTRY_SIZE;
    int entry_num = NORMAL_JMP_ENTRY_NUM;
    if (table->cur + entry_num > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, entry_num);
        return 0;
    }

    jmp[index] = jmp_addr;
    jmp[index + 1] = tls_addr;
    table->cur += entry_num;

    log_debug("\tsetup got table 0x%lx -> 0x%lx, tls_addr=0x%lx\n",
        entry_addr, jmp_addr, tls_addr);

    return entry_addr;
}

unsigned long insert_plt_table(struct patch_context *ctx, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    // elf_addr = setup_jmp_table(ctx, jmp_addr, false);
    elf_addr = setup_jmp_table_with_plt(ctx, jmp_addr, (unsigned long)(uintptr_t)addr);

    log_debug("jump: 0x%lx: jmp_addr=0x%lx, tls_addr=0x%lx\n",
        elf_addr, jmp_addr, tls_addr);

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
     * Addr with this type means the symbol is a dynamic TLS variable.
     * Addr points to a GOT entry(16 bytes) having type
     *
     * typedef struct {
     *		unsigned long int ti_module;
     *		unsigned long int ti_offset;
     * } tls_index;
     *
     * We also need copy ti_offset to our jump table.
     *
     * The corresponding symbol will associate with TLS_GD_HI20
     * relocation type, using this tls_index as argument to call
     * `void *__tls_get_addr (tls_index *ti)` to resolve the real address.
     */
    if (r_type == R_RISCV_TLS_DTPMOD64 &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(tls_addr))) {
        log_err("copy address failed\n");
        goto out;
    }

    elf_addr = setup_got_table(ctx, jmp_addr, tls_addr);

out:
    return elf_addr;
}

/*
 * The patch is a .o file, has only static relocations, all symbols
 * have been resolved with our jump table act as got/plt.
 */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wimplicit-fallthrough"
int apply_relocate_add(struct patch_context *ctx, unsigned int relsec)
{
    Elf_Shdr *shdrs = ctx->shdrs;
    const char *strtab = (void *)ctx->strtab_shdr->sh_addr;
    unsigned int i;
    Elf_Sym *sym, *symtab;
    char const *sym_name;
    void *reloc_place;
    void *ureloc_place;
    u64 sym_addr;
    u64 got;
    Elf_Rela *rel = (void *)shdrs[relsec].sh_addr;
    unsigned int reloc_sec = shdrs[relsec].sh_info;

    symtab = (void *)ctx->symtab_shdr->sh_addr;
    // sh_addr = kdest, is the section start in hot patch kalloc memory
    // sh_addralign = dest, is the section start in VMA pole
    void *sec_kaddr = (void *)shdrs[reloc_sec].sh_addr;
    void *sec_vaddr = (void *)shdrs[reloc_sec].sh_addralign;
    log_debug("Applying relocate section %u to %u\n", relsec, reloc_sec);
    log_debug("sec_kaddr = 0x%llx sec_vaddr = 0x%llx\n", (u64)sec_kaddr, (u64)sec_vaddr);
    
    for (i = 0; i < shdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* corresponds to P in the kernel space */
        reloc_place = (void *)sec_kaddr + rel[i].r_offset;

        /* corresponds to P in user space */
        ureloc_place = (void *)sec_vaddr + rel[i].r_offset;

        /* sym is the ELF symbol we're referring to */
        sym = &symtab[ELF_R_SYM(rel[i].r_info)];
        sym_name = strtab + sym->st_name;

        /* val corresponds to (S + A) */
        sym_addr = (s64)(sym->st_value + rel[i].r_addend);
        log_debug("'%s'\t type %d r_offset=0x%llx, st_value=0x%llx, r_addend=0x%llx\n",
            sym_name, (int)ELF_R_TYPE(rel[i].r_info), rel[i].r_offset, sym->st_value, rel[i].r_addend);
        log_debug("\t(S + A) = 0x%llx \tP(kernel) = 0x%Lx \tP(user) = 0x%Lx\n",
            sym_addr, (u64)reloc_place, (u64)ureloc_place);
        log_debug("\t(before) *reloc_place = 0x%llx\n", *(u64*)reloc_place);

        /* Perform the static relocation. */
        switch (ELF_R_TYPE(rel[i].r_info)) {
            case R_RISCV_NONE:
            case R_RISCV_TPREL_ADD:
                break;

            case R_RISCV_64:
                *(unsigned long *)reloc_place = sym_addr;
                break;

            /* seems no need to recalculate as it should confined in the same func */
            case R_RISCV_BRANCH:
                sym_addr -= (unsigned long)ureloc_place;
                if ((signed)sym_addr >= 4096 || (signed)sym_addr < -4096)
                    goto overflow;
                *(unsigned *)reloc_place = set_btype_imm(*(unsigned *)reloc_place, sym_addr);
                break;

            case R_RISCV_JAL:
                sym_addr -= (unsigned long)ureloc_place;
                if ((signed)sym_addr >= (1 << 20) || (signed)sym_addr < -(1 << 20))
                    goto overflow;
                *(unsigned *)reloc_place = set_jtype_imm(*(unsigned *)reloc_place, sym_addr);
                break;

            case R_RISCV_CALL:
            case R_RISCV_CALL_PLT:
                // in our jump table, must not overflow
                sym_addr -= (unsigned long)ureloc_place;
                *(unsigned *)reloc_place = set_utype_imm(*(unsigned *)reloc_place, sym_addr);
                *(unsigned *)(reloc_place + 4) = set_itype_imm(*(unsigned *)(reloc_place + 4), sym_addr);
                break;

            case R_RISCV_GOT_HI20:
            case R_RISCV_TLS_GOT_HI20:
            case R_RISCV_TLS_GD_HI20:
                got = get_or_setup_got_entry(ctx, sym);
                if (got == 0) {
                    goto overflow;
                }
                sym_addr = got + rel[i].r_addend; // G + GOT + A;
            case R_RISCV_PCREL_HI20:
                sym_addr -= (unsigned long)ureloc_place; // fall through
            case R_RISCV_HI20:
            case R_RISCV_TPREL_HI20:
                if ((long)sym_addr != (long)(int)sym_addr)
                    goto overflow;
                *(unsigned *)reloc_place = set_utype_imm(*(unsigned *)reloc_place, sym_addr);
                break;

            case R_RISCV_PCREL_LO12_I:
                sym_addr = find_pcrel_hi_value(rel + i, i, symtab, sym->st_value - (Elf64_Addr)sec_vaddr);
                if (sym_addr == 0)
                    goto overflow;
                sym_addr -= sym->st_value; // fall through
            case R_RISCV_LO12_I:
            case R_RISCV_TPREL_LO12_I:
                *(unsigned *)reloc_place = set_itype_imm(*(unsigned *)reloc_place, sym_addr);
                break;

            case R_RISCV_PCREL_LO12_S:
                sym_addr = find_pcrel_hi_value(rel + i, i, symtab, sym->st_value - (Elf64_Addr)sec_vaddr);
                if (sym_addr == 0)
                    goto overflow;
                sym_addr -= sym->st_value; // fall through
            case R_RISCV_LO12_S:
            case R_RISCV_TPREL_LO12_S:
                *(unsigned *)reloc_place = set_stype_imm(*(unsigned *)reloc_place, sym_addr);
                break;

            /* inner function label calculation, must not overflow */
            case R_RISCV_ADD8:
                *(char *)reloc_place += sym_addr;
                break;
            case R_RISCV_ADD16:
                *(short *)reloc_place += sym_addr;
                break;
            case R_RISCV_ADD32:
                *(int *)reloc_place += sym_addr;
                break;
            case R_RISCV_ADD64:
                *(long *)reloc_place += sym_addr;
                break;

            case R_RISCV_SUB8:
                *(char *)reloc_place -= sym_addr;
                break;
            case R_RISCV_SUB16:
                *(short *)reloc_place -= sym_addr;
                break;
            case R_RISCV_SUB32:
                *(int *)reloc_place -= sym_addr;
                break;
            case R_RISCV_SUB64:
                *(long *)reloc_place -= sym_addr;
                break;

            case R_RISCV_RVC_BRANCH:
                sym_addr -= (unsigned long)ureloc_place;
                if ((signed)sym_addr >= 256 || (signed)sym_addr < -256)
                    goto overflow;
                *(unsigned short *)reloc_place = set_cbtype_imm(*(unsigned short *)reloc_place, sym_addr);
                break;

            case R_RISCV_RVC_JUMP:
                sym_addr -= (unsigned long)ureloc_place;
                if ((signed)sym_addr >= 2048 || (signed)sym_addr < -2048)
                    goto overflow;
                *(unsigned short *)reloc_place = set_cjtype_imm(*(unsigned short *)reloc_place, sym_addr);
                break;

            case R_RISCV_RVC_LUI:
                if ((signed)sym_addr >= (1 << 17) || (signed)sym_addr < -(1 << 17) || (sym_addr & 0x3f000) == 0)
                    goto overflow;
                *(unsigned short *)reloc_place = set_citype_imm(*(unsigned short *)reloc_place, sym_addr);
                break;

            case R_RISCV_SET8:
                *(char *)reloc_place = sym_addr;
                break;
            case R_RISCV_SET16:
                *(short *)reloc_place = sym_addr;
                break;
            case R_RISCV_32_PCREL:
            case R_RISCV_PLT32:
                sym_addr -= (unsigned long)ureloc_place; // fall through
            case R_RISCV_32:
            case R_RISCV_SET32:
                if ((long)sym_addr != (long)(int)sym_addr)
                    goto overflow;
                *(int *)reloc_place = sym_addr;
                break;

            case R_RISCV_SUB6:
                char w6 = (*(char *)reloc_place - (char)sym_addr) & 0x3f;
                *(char *)reloc_place = (*(char *)reloc_place & 0xc0) | w6;
                break;
            case R_RISCV_SET6:
                *(char *)reloc_place = (*(char *)reloc_place & 0xc0) | (sym_addr & 0x3f);
                break;

            default:
                log_err("upatch: unsupported RELA relocation: %llu\n",
                    ELF_R_TYPE(rel[i].r_info));
                return -ENOEXEC;
        }
        log_debug("\t(after) *reloc_place = 0x%llx\n", *(u64*)reloc_place);
    }
    return 0;

overflow:
    log_err("upatch: overflow in relocation type %d sym_addr %llx\n",
              (int)ELF_R_TYPE(rel[i].r_info), sym_addr);
    return -ENOEXEC;
}
#pragma GCC diagnostic pop


bool is_got_rela_type(int type)
{
    switch (type) {
        case R_RISCV_GOT_HI20:
        case R_RISCV_TLS_GOT_HI20:
        case R_RISCV_TLS_GD_HI20:
            return true;
        default:
            return false;
    }
}
#endif /* __riscv */
