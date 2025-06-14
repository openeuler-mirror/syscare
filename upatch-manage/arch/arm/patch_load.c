// SPDX-License-Identifier: GPL-2.0
/*
 * setup jmp table and do relocation in arm
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

#ifdef __arm__

#include <linux/uaccess.h>
#include <asm/hwcap.h>

#include "../../util.h"
#include "../../patch_load.h"
#include "../patch_load.h"

#ifndef R_ARM_TLS_DESC
#define R_ARM_TLS_DESC 13
#endif

#ifndef R_ARM_GLOB_DAT
#define R_ARM_GLOB_DAT 21
#endif

#ifndef R_ARM_JUMP_SLOT
#define R_ARM_JUMP_SLOT 22
#endif

#ifndef R_ARM_GOTOFF32
#define R_ARM_GOTOFF32 24
#endif
#ifndef R_ARM_GOTPC
#define R_ARM_GOTPC 25
#endif
#ifndef R_ARM_GOT32
#define R_ARM_GOT32 26
#endif
#ifndef R_ARM_TLS_GOTDESC
#define R_ARM_TLS_GOTDESC 90
#endif
#ifndef R_ARM_GOT_ABS
#define R_ARM_GOT_ABS 95
#endif
#ifndef R_ARM_GOT_PREL
#define R_ARM_GOT_PREL 96
#endif
#ifndef R_ARM_GOT_BREL12
#define R_ARM_GOT_BREL12 97
#endif
#ifndef R_ARM_GOTOFF12
#define R_ARM_GOTOFF12 98
#endif
#ifndef R_ARM_GOTRELAX
#define R_ARM_GOTRELAX 99
#endif
#ifndef R_ARM_TLS_GD32
#define R_ARM_TLS_GD32 104
#endif
#ifndef R_ARM_TLS_LDM32
#define R_ARM_TLS_LDM32 105
#endif
#ifndef R_ARM_TLS_IE32
#define R_ARM_TLS_IE32 107
#endif
#ifndef R_ARM_TLS_IE12GP
#define R_ARM_TLS_IE12GP 111
#endif
#ifndef R_ARM_THM_GOT_BREL12
#define R_ARM_THM_GOT_BREL12 131
#endif

/* arm32 PC = crr_addr+8
 * r12 is temp data storage register, no need to restore
 * 0:   ldr r12, [pc]
 * 4:   mov pc, r12             //bx r12
 * 8:   <got_addr>
*/
#define ARM_JUMP_TABLE_JMP_1 0xE59FC000
#define ARM_JUMP_TABLE_JMP_2 0xE1A0F00C

/* For IFUNC(indirect function), the symbol value is point to the resolve function
 * We should call resolve func and get the real func address in x0
 * For armv7 and sparc64, we need to pass hwcap as first arg to resolve function
 * 0x00 push {r0, r1, r2, lr}   save arg for IFUNC, like memcpy
 * 0x04 ldr r12, [pc, #0x14]    load resolve func addr
   0x08 ldr r0, [pc, #0x14]     load hwcap
 * 0xoc mov lr, pc              save return addr 0x14
 * 0x10 mov pc, r12             jmp to resolve func
 * 0x14 mov r12, r0             get the real addr of IFUNC
 * 0x18 pop {r0, r1, r2, lr}    restore arg for IFUNC
 * 0x1c mov pc, r12             jmp to real IFUNC
 * 0x20 addr[0]
 * 0x24 hwcap
 */

#define ARM_CALL_IFUNC_1 0xE92D4007
#define ARM_CALL_IFUNC_2 0xE59FC014
#define ARM_CALL_IFUNC_3 0xE59F0014
#define ARM_CALL_IFUNC_4 0xE1A0E00F
#define ARM_CALL_IFUNC_5 0xE1A0F00C
#define ARM_CALL_IFUNC_6 0xE1A0C000
#define ARM_CALL_IFUNC_7 0xE8BD4007
#define ARM_CALL_IFUNC_8 0xE1A0F00C

enum arm_reloc_op {
    RELOC_OP_NONE,
    RELOC_OP_ABS,
    RELOC_OP_PREL,
};

unsigned long setup_jmp_table(struct upatch_info *info, unsigned long jmp_addr, bool is_ifunc)
{
    struct jmp_table *table = &info->layout.table;
    unsigned long *jmp = info->layout.kbase + table->off;
    unsigned int index = table->cur;
    int entry_num = is_ifunc ? IFUNC_JMP_ENTRY_NUM : NORMAL_JMP_ENTRY_NUM;
    if (table->cur + entry_num > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, entry_num);
        return 0;
    }

    if (is_ifunc) {
        jmp[index]      = ARM_CALL_IFUNC_1;
        jmp[index + 1]  = ARM_CALL_IFUNC_2;
        jmp[index + 2]  = ARM_CALL_IFUNC_3;
        jmp[index + 3]  = ARM_CALL_IFUNC_4;
        jmp[index + 4]  = ARM_CALL_IFUNC_5;
        jmp[index + 5]  = ARM_CALL_IFUNC_6;
        jmp[index + 6]  = ARM_CALL_IFUNC_7;
        jmp[index + 7]  = ARM_CALL_IFUNC_8;
        jmp[index + 8]  = jmp_addr;
        jmp[index + 9]  = ELF_HWCAP;
    } else {
        jmp[index]      = ARM_JUMP_TABLE_JMP_1;
        jmp[index + 1]  = ARM_JUMP_TABLE_JMP_2;
        jmp[index + 2]  = jmp_addr;
    }
    table->cur += entry_num;

    return info->layout.base + table->off + index * JMP_ENTRY_SIZE;
}

unsigned long setup_got_table(struct upatch_info *info, unsigned long jmp_addr, unsigned long tls_addr)
{
    struct jmp_table *table = &info->layout.table;
    unsigned long *jmp = info->layout.kbase + table->off;
    unsigned int index = table->cur;
    unsigned long entry_addr = info->layout.base + table->off + index * JMP_ENTRY_SIZE;
    int entry_num = NORMAL_JMP_ENTRY_NUM;
    if (table->cur + entry_num > table->max) {
        log_err("jmp table overflow, cur = %d, max = %d, num = %d\n",
            table->cur, table->max, entry_num);
        return 0;
    }

    jmp[index] = jmp_addr;
    jmp[index + 1] = tls_addr;
    table->cur += entry_num;

    log_debug("\tsetup got table 0x%lx -> 0x%lx, tls_addr=0x%lx\n", entry_addr, jmp_addr, tls_addr);

    return entry_addr;
}

unsigned long insert_plt_table(struct upatch_info *info, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    if (r_type == R_ARM_TLS_DESC &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    if (r_type == R_ARM_TLS_DESC) {
        elf_addr = setup_got_table(info, jmp_addr, tls_addr);
    } else {
        elf_addr = setup_jmp_table(info, jmp_addr, 0);
    }

    log_debug("jump: 0x%lx: jmp_addr=0x%lx, tls_addr=0x%lx\n", elf_addr, jmp_addr, tls_addr);

out:
    return elf_addr;
}

unsigned long insert_got_table(struct upatch_info *info, unsigned long r_type, void __user *addr)
{
    unsigned long jmp_addr;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (copy_from_user((void *)&jmp_addr, addr, sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    if (r_type == R_ARM_TLS_DESC &&
        copy_from_user((void *)&tls_addr, addr + sizeof(unsigned long), sizeof(unsigned long))) {
        log_err("copy address failed\n");
        goto out;
    }

    elf_addr = setup_got_table(info, jmp_addr, tls_addr);

out:
    return elf_addr;
}

static u32 calc_reloc(enum arm_reloc_op op, u32 *place, u32 S)
{
    s32 sval = 0;
    switch (op) {
        case RELOC_OP_ABS:
            // S + A
            sval = S;
            break;
        case RELOC_OP_PREL:
            // S + A - P
            sval = S - (u32)place;
            break;
        default:
            log_err("\tunknown relocation operation %d\n", op);
            break;
    }

    log_debug("\tS + A = 0x%x, P = 0x%x, X = 0x%x\n", S, (u32)place, sval);
    return sval;
}

int apply_relocate_add(struct upatch_info *info, unsigned int relsec)
{
    Elf_Shdr *shdrs = info->shdrs;
    const char *strtab = info->strtab;
    unsigned int symindex = info->index.sym;
    unsigned int i;
    Elf_Sym *sym;
    char const *sym_name;
    u32 *reloc_place;
    u32 *ureloc_place;
    u32 sym_addr;
    u32 got;
    u32 tmp;
    s32 result;
    Elf_Rel *rel = (void *)shdrs[relsec].sh_addr;
    u32 got_vaddr = 0;
    struct jmp_table *table;
    unsigned int reloc_sec = shdrs[relsec].sh_info;

    // sh_addr = kdest, is the section start in hot patch kalloc memory
    // sh_addralign = dest, is the section start in VMA pole
    u32 sec_kaddr = shdrs[reloc_sec].sh_addr;
    u32 sec_vaddr = shdrs[reloc_sec].sh_addralign;

    log_debug("sec_kaddr = 0x%x sec_vaddr = 0x%x\n", sec_kaddr, sec_vaddr);

    for (i = 0; i < shdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* relocP corresponds to P in the kernel space */
        reloc_place = (void *)sec_kaddr + rel[i].r_offset;
        /* urelocP corresponds to P in user spcace */
        ureloc_place = (void *)sec_vaddr + rel[i].r_offset;
        /* sym is the ELF symbol we're referring to */
        sym = (Elf_Sym *)shdrs[symindex].sh_addr + ELF_R_SYM(rel[i].r_info);
        sym_name = strtab + sym->st_name;

        sym_addr = sym->st_value;
        log_debug("'%s'\t type %d r_offset=0x%x, st_value=0x%x\n",
            sym_name, (int)ELF_R_TYPE(rel[i].r_info), rel[i].r_offset, sym->st_value);
        log_debug("\t(S + A) = 0x%x \tP(kernel) = 0x%x \tP(user) = 0x%x\n",
            sym_addr, (u32)reloc_place, (u32)ureloc_place);
        log_debug("\t(before) *reloc_place = 0x%x\n", *reloc_place);

        table = &info->layout.table;
        got_vaddr = info->layout.base + table->off;

        switch (ELF_R_TYPE(rel[i].r_info)) {
            case R_ARM_NONE:
                break;
            case R_ARM_ABS32:
            case R_ARM_TARGET1: // (S + A) | T
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                *reloc_place += result;
                break;

            case R_ARM_PC24:
            case R_ARM_CALL:
            case R_ARM_JUMP24:
                if (sym_addr & 3) {
                    log_err("unsupported interworking call (ARM -> Thumb)\n");
                    return -ENOEXEC;
                }
                result = __mem_to_opcode_arm(*reloc_place);
                result = (result & 0x00ffffff) << 2;
                if (result & 0x02000000)
                    result -= 0x04000000;

                // L = S + A - P (A = 0)
                result += sym_addr - (u32)ureloc_place;
                /*
                    * Route through a PLT entry if 'offset' exceeds the
                    * supported range. Note that 'offset + loc + 8'
                    * contains the absolute jump target, i.e.,
                    * @sym + addend, corrected for the +8 PC bias.
                    */
                if (IS_ENABLED(CONFIG_ARM_MODULE_PLTS) &&
                    (result <= (s32)0xfe000000 || result >= (s32)0x02000000)) {
                    result = setup_jmp_table(info, result + (u32)ureloc_place + 8, false)
                        - (u32)ureloc_place - 8;
                    if (!result) {
                        goto overflow;
                    }
                    log_warn("setup jmp table for PLT in arm! result = 0x%x\n", result);
                }

                // check if plt addr still outside of 32MB range
                if (result <= (s32)0xfe000000 || result >= (s32)0x02000000) {
                    log_err("setup jmp table outside of 32MB range for result = 0x%x\n", result);
                    goto overflow;
                }

                result >>= 2;
                result &= 0x00ffffff;

                *reloc_place &= __opcode_to_mem_arm(0xff000000);
                *reloc_place |= __opcode_to_mem_arm(result);
                break;

            case R_ARM_V4BX:
                /* Preserve Rm and the condition code. Alter
                    * other bits to re-code instruction as
                    * MOV PC,Rm.
                    */
                *reloc_place &= __opcode_to_mem_arm(0xf000000f);
                *reloc_place |= __opcode_to_mem_arm(0x01a0f000);
                break;

            case R_ARM_PREL31: /* sign extend */
                result = (*(s32 *)reloc_place << 1) >> 1;
                result += calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                if (result >= 0x40000000 || result < -0x40000000) {
                    goto overflow;
                }
                *reloc_place &= 0x80000000;
                *reloc_place |= (result & 0x7fffffff);
                break;

            case R_ARM_REL32: // ((S + A) | T) - P, T is 0 because we don't have thumb mode
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, sym_addr);
                *reloc_place += result;
                break;

            case R_ARM_MOVW_ABS_NC:     // (S + A) | T
            case R_ARM_MOVT_ABS:        // S + A
            case R_ARM_MOVW_PREL_NC:    // ((S + A) | T) - P
            case R_ARM_MOVT_PREL:       // (S + A) - P
                result = tmp = __mem_to_opcode_arm(*reloc_place);
                result = ((result & 0xf0000) >> 4) | (result & 0xfff);
                result = (result ^ 0x8000) - 0x8000;

                result += sym_addr; // S
                if (ELF_R_TYPE(rel[i].r_info) == R_ARM_MOVT_PREL ||
                    ELF_R_TYPE(rel[i].r_info) == R_ARM_MOVW_PREL_NC)
                    result -= (u32)ureloc_place; // - P
                if (ELF_R_TYPE(rel[i].r_info) == R_ARM_MOVT_ABS ||
                    ELF_R_TYPE(rel[i].r_info) == R_ARM_MOVT_PREL)
                    result >>= 16;

                tmp &= 0xfff0f000;
                tmp |= ((result & 0xf000) << 4) | (result & 0x0fff);

                *reloc_place = __opcode_to_mem_arm(tmp);
                break;

            // The relocation above is implement based on Linux kernel arch/arm/kernel/module.c
            // The relocation below is implement based on LLVM
            case R_ARM_GLOB_DAT:
            case R_ARM_JUMP_SLOT:   // (S + A) | T
                result = calc_reloc(RELOC_OP_ABS, ureloc_place, sym_addr);
                *reloc_place += result;
                break;

            case R_ARM_GOTPC:   // R_ARM_BASE_PREL B(S) + A - P
                // B(S) is the start address of .got section, which is got_vaddr
                log_debug("\t(GOT) got_vaddr = 0x%x\n", got_vaddr);
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, got_vaddr);
                *reloc_place += result;
                break;

            case R_ARM_GOT32:   // R_ARM_GOT_BREL GOT(S) + A - GOT_ORG
                // GOT_ORG is the start address of got table, which is got_vaddr
                log_debug("\t(GOT) got_vaddr = 0x%x\n", got_vaddr);
                got = get_or_setup_got_entry(info, sym);
                if (got == 0) {
                    goto overflow;
                }
                result = calc_reloc(RELOC_OP_PREL, (u32 *)got_vaddr, got);
                *reloc_place += result;
                break;

            case R_ARM_GOT_PREL:    // GOT(S) + A -P
                log_debug("\t(GOT) got_vaddr = 0x%x\n", got_vaddr);
                got = get_or_setup_got_entry(info, sym);
                if (got == 0) {
                    goto overflow;
                }
                result = calc_reloc(RELOC_OP_PREL, ureloc_place, got);
                *reloc_place += result;
                break;

            default:
                log_debug("\tunsupported REL relocation: %u\n", ELF_R_TYPE(rel[i].r_info));
                return -ENOEXEC;
        }
        log_debug("\t(after) *reloc_place = 0x%x, result = 0x%x\n", *reloc_place, result);
    }
    return 0;

overflow:
    log_err("\toverflow in relocation type %d val %x reloc 0x%x\n",
        (int)ELF_R_TYPE(rel[i].r_info), sym_addr, result);
    return -ENOEXEC;
}

bool is_got_rela_type(int type)
{
    switch (type) {
        case R_ARM_GOTOFF32:
        case R_ARM_GOT32:
        case R_ARM_TLS_GOTDESC:
        case R_ARM_GOT_ABS:
        case R_ARM_GOT_PREL:
        case R_ARM_GOT_BREL12:
        case R_ARM_GOTOFF12:
        case R_ARM_GOTRELAX:
        case R_ARM_TLS_GD32:
        case R_ARM_TLS_LDM32:
        case R_ARM_TLS_IE32:
        case R_ARM_TLS_IE12GP:
        case R_ARM_THM_GOT_BREL12:
            return true;
            break;
        default:
            return false;
            break;
    }
    return false;
}

#endif /* __arm__ */
