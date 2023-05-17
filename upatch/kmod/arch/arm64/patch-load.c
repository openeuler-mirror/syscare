// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   renoseven <dev@renoseven.net>
 *
 */

#ifdef __aarch64__

#include "arch/patch-load.h"
#include "arch/arm64/insn.h"

/* ldr x17, #8
 * br x17
 */
#define AARCH64_JUMP_TABLE_JMP 0xd61f022058000051

enum aarch64_reloc_op {
    RELOC_OP_NONE,
    RELOC_OP_ABS,
    RELOC_OP_PREL,
    RELOC_OP_PAGE,
};

unsigned long jmp_table_inst()
{
    return AARCH64_JUMP_TABLE_JMP;
}

void setup_parameters(struct pt_regs *regs, unsigned long para_a,
    unsigned long para_b, unsigned long para_c)
{
    regs->regs[0] = para_a;
    regs->regs[1] = para_b;
    regs->regs[2] = para_c;
}

static inline s64 calc_reloc(enum aarch64_reloc_op op, void *place, u64 val)
{
    s64 sval;
    switch (op) {
    case RELOC_OP_ABS:
        // S + A
        sval = val;
        break;
    case RELOC_OP_PREL:
        // S + A - P
        sval = val - (u64)place;
        break;
    case RELOC_OP_PAGE:
        // Page(S + A) - Page(P)
        sval = (val & ~0xfff) - ((u64)place & ~0xfff);
        break;
    default:
        pr_err("upatch: unknown relocation operation %d\n", op);
        break;
    }

    pr_debug("upatch: reloc, S+A=0x%llx, P=0x%llx, X=0x%llx", val, (u64)place, sval);
    return sval;
}

static inline u64 extract_insn_imm(s64 sval, int len, int lsb)
{
    u64 imm, imm_mask;

    imm = sval >> lsb;
    imm_mask = (BIT(lsb + len) - 1) >> lsb;
    imm = imm & imm_mask;

    pr_debug("upatch: extract imm, X=0x%llx, X[%d:%d]=0x%llx", sval, (len + lsb - 1), lsb, imm);
    return imm;
}

static inline u32 insert_insn_imm(enum aarch64_insn_imm_type imm_type, void *place, u64 imm)
{
    u32 insn, new_insn;

    insn = le32_to_cpu(*(__le32 *)place);
    new_insn = aarch64_insn_encode_immediate(imm_type, insn, imm);

    pr_debug("upatch: insert imm, P=0x%llx, insn=0x%x, imm_type=%d, imm=0x%llx, new_insn=0x%x",
             (u64)place, insn, imm_type, imm, new_insn);
    return new_insn;
}

int apply_relocate_add(struct upatch_load_info *info, Elf64_Shdr *sechdrs,
    const char *strtab, unsigned int symindex,
    unsigned int relsec, struct upatch_module *me)
{
    unsigned int i;
    Elf64_Sym *sym;
    char const *sym_name;
    void *loc;
    void *uloc;
    u64 val;
    s64 result;
    Elf64_Rela *rel = (void *)sechdrs[relsec].sh_addr;

    for (i = 0; i < sechdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* loc corresponds to P in the kernel space */
        loc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addr
            + rel[i].r_offset;

        /* uloc corresponds P in user space */
        uloc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addralign
            + rel[i].r_offset;

        /* sym is the ELF symbol we're referring to */
        sym = (Elf64_Sym *)sechdrs[symindex].sh_addr
            + ELF64_R_SYM(rel[i].r_info);
        sym_name = strtab + sym->st_name;

        /* val corresponds to (S + A) */
        val = (s64)(sym->st_value + rel[i].r_addend);
        pr_debug("upatch: reloc symbol, name=%s, k_addr=0x%llx, u_addr=0x%llx, r_offset=0x%llx, st_value=0x%llx, r_addend=0x%llx",
                 sym_name,
                 (unsigned long long)sechdrs[sechdrs[relsec].sh_info].sh_addr,
                 (unsigned long long)sechdrs[sechdrs[relsec].sh_info].sh_addralign,
                 rel[i].r_offset, sym->st_value, rel[i].r_addend);

        /* Perform the static relocation. */
        switch (ELF64_R_TYPE(rel[i].r_info)) {
        /* Null relocations. */
        case R_ARM_NONE:
        case R_AARCH64_NONE:
            break;
        /* Data relocations. */
        case R_AARCH64_ABS64:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            *(s64 *)loc = result;
            break;
        case R_AARCH64_ABS32:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            if (result < S32_MIN || result > S32_MAX) {
                goto overflow;
            }
            *(s32 *)loc = result;
            break;
        case R_AARCH64_ABS16:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            if (result < S16_MIN || result > S16_MAX) {
                goto overflow;
            }
            *(s16 *)loc = result;
            break;
        case R_AARCH64_PREL64:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            *(s64 *)loc = result;
            break;
        case R_AARCH64_PREL32:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            if (result < S32_MIN || result > S32_MAX) {
                goto overflow;
            }
            *(s32 *)loc = result;
            break;
        case R_AARCH64_PREL16:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            if (result < S32_MIN || result > S32_MAX) {
                goto overflow;
            }
            *(s16 *)loc = result;
            break;
        /* Immediate instruction relocations. */
        case R_AARCH64_LD_PREL_LO19:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            // TODO: ovf check -2^20 < X < 2^20
            result = extract_insn_imm(result, 19, 2);
            result = insert_insn_imm(AARCH64_INSN_IMM_19, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_ADR_PREL_LO21:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            // TODO: ovf check -2^20 < X < 2^20
            result = extract_insn_imm(result, 21, 0);
            result = insert_insn_imm(AARCH64_INSN_IMM_ADR, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_ADR_PREL_PG_HI21:
            result = calc_reloc(RELOC_OP_PAGE, uloc, val);
            if (result < S32_MIN || result > S32_MAX) {
                goto overflow;
            }
            result = extract_insn_imm(result, 21, 12);
            result = insert_insn_imm(AARCH64_INSN_IMM_ADR, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_ADR_PREL_PG_HI21_NC:
            result = calc_reloc(RELOC_OP_PAGE, uloc, val);
            result = extract_insn_imm(result, 21, 12);
            result = insert_insn_imm(AARCH64_INSN_IMM_ADR, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_ADD_ABS_LO12_NC:
        case R_AARCH64_LDST8_ABS_LO12_NC:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            result = extract_insn_imm(result, 12, 0);
            result = insert_insn_imm(AARCH64_INSN_IMM_12, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_LDST16_ABS_LO12_NC:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            result = extract_insn_imm(result, 11, 1);
            result = insert_insn_imm(AARCH64_INSN_IMM_12, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_LDST32_ABS_LO12_NC:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            result = extract_insn_imm(result, 10, 2);
            result = insert_insn_imm(AARCH64_INSN_IMM_12, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_LDST64_ABS_LO12_NC:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            result = extract_insn_imm(result, 9, 3);
            result = insert_insn_imm(AARCH64_INSN_IMM_12, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_LDST128_ABS_LO12_NC:
            result = calc_reloc(RELOC_OP_ABS, uloc, val);
            result = extract_insn_imm(result, 8, 4);
            result = insert_insn_imm(AARCH64_INSN_IMM_12, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_TSTBR14:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            // TODO: ovf check -2^15 < X < 2^15
            result = extract_insn_imm(result, 14, 2);
            result = insert_insn_imm(AARCH64_INSN_IMM_14, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_CONDBR19:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            result = extract_insn_imm(result, 19, 2);
            result = insert_insn_imm(AARCH64_INSN_IMM_19, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;
        case R_AARCH64_JUMP26:
        case R_AARCH64_CALL26:
            result = calc_reloc(RELOC_OP_PREL, uloc, val);
            // TODO: ovf check -2^27 < X < 2^27
            result = extract_insn_imm(result, 26, 2);
            result = insert_insn_imm(AARCH64_INSN_IMM_26, loc, result);
            *(__le32 *)loc = cpu_to_le32(result);
            break;

        default:
            pr_err("upatch: unsupported RELA relocation: %llu\n",
                   ELF64_R_TYPE(rel[i].r_info));
            return -ENOEXEC;
        }
    }
    return 0;

overflow:
    pr_err("upatch: overflow in relocation type %d val %Lx reloc %llx\n",
        (int)ELF64_R_TYPE(rel[i].r_info), val, result);
    return -ENOEXEC;
}

#endif /* __aarch64__ */
