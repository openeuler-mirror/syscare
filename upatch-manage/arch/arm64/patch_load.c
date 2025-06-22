// SPDX-License-Identifier: GPL-2.0
/*
 * setup jmp table and do relocation in arm64
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

#ifdef __aarch64__

#include <linux/uaccess.h>

#include "../../util.h"
#include "../patch_load.h"
#include "insn.h"

/* For UND function, we need to find its real addr in VMA, after that, we need to
 * create plt.got table to store instruction to jmp to it real addr
 * 0:   ldr x17, #8
 * 4:   br x17
 * 8:   <addr>
 * 12:  <addr>
 */
#define AARCH64_JUMP_TABLE_JMP 0xd61f022058000051

/* For IFUNC(indirect function), the symbol value is point to the resolve function
 * We should call resolve func and get the real func address in x0
 * We save x0, x1, x2, x30, because IFUNC will only use 3 args, and x30 is the LR store the return addr
 * 0:   stp x0, x1, [sp, #-32]!
 * 4:   stp x2, x30, [sp, #16]
 * 8:   ldr x17, #0x18
 * C:   blr x17
 * 10:  mov x17, x0
 * 14:  ldp x2, x30, [sp, #16]
 * 18:  ldp x0, x1, [sp], #32
 * 1C:  br x17
 * 20:  <addr>
 * 24:  <addr>
 */

#define AARCH64_CALL_IFUNC_1 0xA9017BE2A9BE07E0
#define AARCH64_CALL_IFUNC_2 0xD63F0220580000D1
#define AARCH64_CALL_IFUNC_3 0xA9417BE2AA0003F1
#define AARCH64_CALL_IFUNC_4 0xD61F0220A8C207E0


/*
 * 0:   ldr x16, #24        load mem from PC+24 into x16, x16 = *(PC+24) = addr[1]
 * 4:   ldr x17, #12        load mem from PC+12 into x17, x17 = *(PC+16) = addr[0]
 * 8:   br x17              jump to addr in x17, jmp tp addr[0]
 * 12:  undefined           we only need 3 instruction, but table entry is 64bit size
 * 16:  addr[0]             jump destination
 * 20:  addr[0]
 * 24:  addr[1]             plt entry address
 * 28:  addr[1]
 */
#define AARCH64_JUMP_TABLE_JMP1 0x58000071580000d0
#define AARCH64_JUMP_TABLE_JMP2 0x00000000d61f0220

#ifndef R_AARCH64_MOVW_GOTOFF_GO
#define R_AARCH64_MOVW_GOTOFF_GO            300
#endif

#ifndef R_AARCH64_LD64_GOTPAGE_LO15
#define R_AARCH64_LD64_GOTPAGE_LO15         313
#endif


#ifndef R_AARCH64_ADR_GOT_PAGE
#define R_AARCH64_ADR_GOT_PAGE              311
#endif

#ifndef R_AARCH64_LD64_GOT_LO12_NC
#define R_AARCH64_LD64_GOT_LO12_NC          312
#endif

#ifndef R_AARCH64_LD64_GOTPAGE_LO15
#define R_AARCH64_LD64_GOTPAGE_LO15         313
#endif

#ifndef R_AARCH64_TLSLE_ADD_TPREL_HI12
#define R_AARCH64_TLSLE_ADD_TPREL_HI12      549
#endif

#ifndef R_AARCH64_TLSLE_ADD_TPREL_LO12_NC
#define R_AARCH64_TLSLE_ADD_TPREL_LO12_NC   551
#endif

#ifndef R_AARCH64_TLSDESC_ADR_PAGE21
#define R_AARCH64_TLSDESC_ADR_PAGE21        562
#endif

#ifndef R_AARCH64_TLSDESC_LD64_LO12
#define R_AARCH64_TLSDESC_LD64_LO12         563
#endif

#ifndef R_AARCH64_TLSDESC_ADD_LO12
#define R_AARCH64_TLSDESC_ADD_LO12          564
#endif

#ifndef R_AARCH64_TLSDESC_CALL
#define R_AARCH64_TLSDESC_CALL              569
#endif

#ifndef R_AARCH64_TLSDESC
#define R_AARCH64_TLSDESC                   1031
#endif

#define TCB_SIZE        2 * sizeof(void *)

enum aarch64_reloc_op {
    RELOC_OP_NONE,
    RELOC_OP_ABS,
    RELOC_OP_PREL,
    RELOC_OP_PAGE,
};

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
        jmp[index]      = AARCH64_CALL_IFUNC_1;
        jmp[index + 1]  = AARCH64_CALL_IFUNC_2;
        jmp[index + 2]  = AARCH64_CALL_IFUNC_3;
        jmp[index + 3]  = AARCH64_CALL_IFUNC_4;
        jmp[index + 4]  = jmp_addr;
    } else {
        jmp[index]      = AARCH64_JUMP_TABLE_JMP;
        jmp[index + 1]  = jmp_addr;
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

    jmp[index] = AARCH64_JUMP_TABLE_JMP1;
    jmp[index + 1]  = AARCH64_JUMP_TABLE_JMP2;
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

    if (r_type == R_AARCH64_TLSDESC &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    if (r_type == R_AARCH64_TLSDESC)
        elf_addr = setup_got_table(ctx, jmp_addr, tls_addr);
    else
        elf_addr = setup_jmp_table_with_plt(ctx, jmp_addr, (unsigned long)(uintptr_t)addr);

    log_debug("jump: 0x%lx: jmp_addr=0x%lx, tls_addr=0x%lx\n",
        elf_addr, jmp_addr, tls_addr);

out:
    return elf_addr;
}

static unsigned long search_insert_plt_table(struct patch_context *ctx,
    unsigned long jmp_addr, unsigned long plt_addr)
{
    struct jmp_table *table = &ctx->layout.table;
    unsigned long *jmp = ctx->layout.kbase + table->off;
    unsigned int i = 0;

    for (i = 0; i < table->max; ++i) {
        if (jmp[i] != jmp_addr) {
            continue;
        }
        return ctx->layout.base + table->off + i * JMP_ENTRY_SIZE;
    }

    return setup_jmp_table_with_plt(ctx, jmp_addr, plt_addr);
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

    if (r_type == R_AARCH64_TLSDESC &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    elf_addr = setup_got_table(ctx, jmp_addr, tls_addr);

out:
    return elf_addr;
}

static s64 calc_reloc(enum aarch64_reloc_op op, void *place, u64 S)
{
    s64 sval = 0;
    switch (op) {
        case RELOC_OP_ABS:
            // S + A
            sval = S;
            break;
        case RELOC_OP_PREL:
            // S + A - P
            sval = S - (u64)place;
            break;
        case RELOC_OP_PAGE:
            // Page(S + A) - Page(P)
            sval = (S & ~0xfff) - ((u64)place & ~0xfff);
            break;
        default:
            log_err("\tunknown relocation operation %d\n", op);
            break;
    }

    log_debug("\tS + A = 0x%llx, P = 0x%llx, X = 0x%llx\n", S, (u64)place, sval);
    return sval;
}

static inline u64 extract_insn_imm(s64 sval, int len, int lsb)
{
    u64 imm;
    u64 imm_mask;

    imm = sval >> lsb;
    imm_mask = (BIT(lsb + len) - 1) >> lsb;
    imm = imm & imm_mask;

    log_debug("\textract imm, X=0x%llx, X[%d:%d]=0x%llx\n", sval, (len + lsb - 1), lsb, imm);
    return imm;
}

static inline u32 insert_insn_imm(enum aarch64_insn_imm_type imm_type, void *place, u64 imm)
{
    u32 insn;
    u32 new_insn;

    insn = le32_to_cpu(*(__le32 *)place);
    new_insn = aarch64_insn_encode_immediate(imm_type, insn, imm);

    log_debug("\tinsert imm, P=0x%llx, insn=0x%x, imm_type=%d, imm=0x%llx, new_insn=0x%x\n",
        (u64)place, insn, imm_type, imm, new_insn);
    return new_insn;
}

int apply_relocate_add(struct patch_context *ctx, unsigned int relsec)
{
    Elf_Shdr *shdrs = ctx->shdrs;
    Elf_Sym *symtab = (void *)ctx->symtab_shdr->sh_addr;
    const char *strtab = (void *)ctx->strtab_shdr->sh_addr;
    unsigned int i;
    Elf_Sym *sym;
    char const *sym_name;
    void *reloc_place;
    void *ureloc_place;
    u64 sym_addr;
    u64 got;
    s64 result;
    u64 got_start = ctx->layout.base + ctx->layout.table.off;
    Elf_Rela *rel = (void *)shdrs[relsec].sh_addr;

    unsigned int reloc_sec = shdrs[relsec].sh_info;

    // sh_addr = kdest, is the section start in hot patch kalloc memory
    // sh_addralign = dest, is the section start in VMA pole
    void *sec_kaddr = (void *)shdrs[reloc_sec].sh_addr;
    void *sec_vaddr = (void *)shdrs[reloc_sec].sh_addralign;
    log_debug("sec_kaddr = 0x%llx sec_vaddr = 0x%llx\n", (u64)sec_kaddr, (u64)sec_vaddr);

    for (i = 0; i < shdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* corresponds to P in the kernel space */
        reloc_place = (void *)sec_kaddr + rel[i].r_offset;

        /* corresponds to P in user space */
        ureloc_place = (void *)sec_vaddr + rel[i].r_offset;

        /* sym is the ELF symbol we're referring to */
        sym = &symtab[ELF_R_SYM(rel[i].r_info)];
        sym_name = strtab + sym->st_name;

        /* src corresponds to (S + A) */
        sym_addr = (s64)(sym->st_value + rel[i].r_addend);
        log_debug("'%s'\t type %d r_offset=0x%llx, st_value=0x%llx, r_addend=0x%llx\n",
            sym_name, (int)ELF_R_TYPE(rel[i].r_info), rel[i].r_offset, sym->st_value, rel[i].r_addend);
        log_debug("\t(S + A) = 0x%llx \tP(kernel) = 0x%Lx \tP(user) = 0x%Lx\n",
            sym_addr, (u64)reloc_place, (u64)ureloc_place);
        log_debug("\t(before) *reloc_place = 0x%llx\n", *(u64*)reloc_place);

        /* Perform the static relocation. */
        switch (ELF_R_TYPE(rel[i].r_info)) {
            /* Null relocations. */
            case R_ARM_NONE:
            case R_AARCH64_NONE:
                break;
            /* Data relocations. */
            case R_AARCH64_ABS64:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                *(s64 *)reloc_place = result;
                break;
            case R_AARCH64_ABS32:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                if (result < -(s64)BIT(31) || result >= (s64)BIT(32))
                    goto overflow;
                *(s32 *)reloc_place = result;
                break;
            case R_AARCH64_ABS16:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                if (result < -(s64)BIT(15) || result >= (s64)BIT(16))
                    goto overflow;
                *(s16 *)reloc_place = result;
                break;
            case R_AARCH64_PREL64:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                *(s64 *)reloc_place = result;
                break;
            case R_AARCH64_PREL32:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result < -(s64)BIT(31) || result >= (s64)BIT(32))
                    goto overflow;
                *(s32 *)reloc_place = result;
                break;
            case R_AARCH64_PREL16:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result < -(s64)BIT(15) || result >= (s64)BIT(16))
                    goto overflow;
                *(s16 *)reloc_place = result;
                break;
            /* Immediate instruction relocations. */
            case R_AARCH64_LD_PREL_LO19:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result < -(s64)BIT(20) || result >= (s64)BIT(20))
                    goto overflow;
                result = extract_insn_imm(result, 19, 2);
                result = insert_insn_imm(AARCH64_INSN_IMM_19, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_ADR_PREL_LO21:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result < -(s64)BIT(20) || result >= (s64)BIT(20))
                    goto overflow;
                result = extract_insn_imm(result, 21, 0);
                result = insert_insn_imm(AARCH64_INSN_IMM_ADR, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_ADR_PREL_PG_HI21:
                result = calc_reloc(RELOC_OP_PAGE, ureloc_place, sym_addr);
                if (result < -(s64)BIT(32) || result >= (s64)BIT(32))
                    goto overflow;
                result = extract_insn_imm(result, 21, 12);
                result = insert_insn_imm(AARCH64_INSN_IMM_ADR, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_ADR_PREL_PG_HI21_NC:
                result = calc_reloc(RELOC_OP_PAGE, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 21, 12);
                result = insert_insn_imm(AARCH64_INSN_IMM_ADR, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_ADD_ABS_LO12_NC:
            case R_AARCH64_LDST8_ABS_LO12_NC:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 12, 0);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LDST16_ABS_LO12_NC:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 11, 1);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LDST32_ABS_LO12_NC:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 10, 2);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LDST64_ABS_LO12_NC:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 9, 3);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LDST128_ABS_LO12_NC:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 8, 4);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TSTBR14:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result < -(s64)BIT(15) || result >= (s64)BIT(15))
                    goto overflow;
                result = extract_insn_imm(result, 14, 2);
                result = insert_insn_imm(AARCH64_INSN_IMM_14, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_CONDBR19:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 19, 2);
                result = insert_insn_imm(AARCH64_INSN_IMM_19, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_JUMP26:
            case R_AARCH64_CALL26:
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                // branch addressable span is +/-128MB
                if (result < -(s64)BIT(27) || result >= (s64)BIT(27)) {
                    log_warn("\tR_AARCH64_CALL26 overflow: result = 0x%llx, uloc = 0x%lx, val = 0x%llx\n",
                        result, (unsigned long)(uintptr_t)ureloc_place, sym_addr);
                    sym_addr = search_insert_plt_table(ctx, sym_addr, (u64)&sym_addr);
                    log_warn("\tR_AARCH64_CALL26 overflow: plt.addr = 0x%llx\n", sym_addr);
                    if (!sym_addr) {
                        goto overflow;
                    }
                    result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                }
                result = extract_insn_imm(result, 26, 2);
                result = insert_insn_imm(AARCH64_INSN_IMM_26, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_ADR_GOT_PAGE:
                got = get_or_setup_got_entry(ctx, sym);
                if (got == 0) {
                    goto overflow;
                }
                result = calc_reloc(RELOC_OP_PAGE, ureloc_place, got);
                if (result < -(s64)BIT(32) || result >= (s64)BIT(32)) {
                    goto overflow;
                }
                result = extract_insn_imm(result, 21, 12);
                result = insert_insn_imm(AARCH64_INSN_IMM_ADR, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LD64_GOT_LO12_NC:
                got = get_or_setup_got_entry(ctx, sym);
                if (got == 0) {
                    goto overflow;
                }
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, got);
                // don't check result & 7 == 0.
                // sometimes, result & 7 != 0, it works fine.
                result = extract_insn_imm(result, 9, 3);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_LD64_GOTPAGE_LO15:
                got = get_or_setup_got_entry(ctx, sym);
                if (got == 0) {
                    goto overflow;
                }
                // G(GDAT(S)) - Page(GOT)
                result = calc_reloc(RELOC_OP_PREL, (void *)(got_start & ~0xfff), got);
                if (result < 0 || result >= (s64)BIT(14)) {
                    log_err("got=%llx got_start=%llx\n", got, got_start);
                    goto overflow;
                }
                result = extract_insn_imm(result, 12, 3);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSLE_ADD_TPREL_HI12:
                result = ALIGN(TCB_SIZE, ctx->target->tls_align) + sym_addr;
                if (result < 0 || result >= BIT(24)) {
                    goto overflow;
                }
                result = extract_insn_imm(result, 12, 12);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSLE_ADD_TPREL_LO12_NC:
                result = ALIGN(TCB_SIZE, ctx->target->tls_align) + sym_addr;
                result = extract_insn_imm(result, 12, 0);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSDESC_ADR_PAGE21:
                result = calc_reloc(RELOC_OP_PAGE, ureloc_place, sym_addr);
                if (result < -(s64)BIT(32) || result >= (s64)BIT(32)) {
                    goto overflow;
                }
                result = extract_insn_imm(result, 21, 12);
                result = insert_insn_imm(AARCH64_INSN_IMM_ADR, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSDESC_LD64_LO12:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                // don't check result & 7 == 0.
                result = extract_insn_imm(result, 9, 3);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSDESC_ADD_LO12:
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                result = extract_insn_imm(result, 12, 0);
                result = insert_insn_imm(AARCH64_INSN_IMM_12, reloc_place, result);
                *(__le32 *)reloc_place = cpu_to_le32(result);
                break;
            case R_AARCH64_TLSDESC_CALL:
                // this is a blr instruction, don't need to modify
                break;
            default:
                log_err("\tunsupported RELA relocation: %llu\n",
                        ELF_R_TYPE(rel[i].r_info));
                return -ENOEXEC;
        }
        log_debug("\t(after) *reloc_place = 0x%llx, result = 0x%llx\n", *(u64*)reloc_place, result);
    }
    return 0;

overflow:
    log_err("\toverflow in relocation type %d val 0x%Lx reloc 0x%llx\n",
        (int)ELF_R_TYPE(rel[i].r_info), sym_addr, result);
    return -ENOEXEC;
}

bool is_got_rela_type(int type)
{
    if (type >= R_AARCH64_MOVW_GOTOFF_GO && type <= R_AARCH64_LD64_GOTPAGE_LO15) {
        return true;
    }
    return false;
}

#endif /* __aarch64__ */
