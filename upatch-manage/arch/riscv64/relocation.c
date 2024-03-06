// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
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

#include <errno.h>

#include "insn.h"
#include "upatch-relocation.h"
#include "upatch-resolve.h"

/*
 * In PCREL_LO12 relocation entity, its corresponding symbol's value
 * points to the ..._HI20 instruction, where the LO12 part of the
 * immediate is part of the ..._HI20 symbol value.
 */
static unsigned long
find_pcrel_hi_value(GElf_Rela *r, int idx, GElf_Sym *st, unsigned long v)
{
    int i = idx;
    r--;
    for (; i > 0; i--, r--) {
        if ((r->r_offset == v) &&
                ((GELF_R_TYPE(r->r_info) == R_RISCV_PCREL_HI20) ||
				 (GELF_R_TYPE(r->r_info) == R_RISCV_TLS_GOT_HI20) ||
				 (GELF_R_TYPE(r->r_info) == R_RISCV_TLS_GD_HI20) ||
				 (GELF_R_TYPE(r->r_info) == R_RISCV_GOT_HI20)))
            return st[GELF_R_SYM(r->r_info)].st_value;
    }

    /* should never happen */
    log_error("Not found no. %d rela's corresponding HI20\n", idx);
    return 0;
}

/*
 * The patch is a .o file, has only static relocations, all symbols
 * have been resolved with our jump table act as got/plt. 
 */
int apply_relocate_add(struct upatch_elf *uelf, unsigned int symindex,
		       unsigned int relsec)
{
	unsigned int i;
	GElf_Sym *sym, *symtab;
	char const *sym_name;
	unsigned long uloc_sec;
	void *loc;
	void *uloc;
	u64 val;
	GElf_Shdr *shdrs = (void *)uelf->info.shdrs;
	GElf_Rela *rel = (void *)shdrs[relsec].sh_addr;
	
	symtab = (GElf_Sym *)shdrs[symindex].sh_addr;
	for (i = 0; i < shdrs[relsec].sh_size / sizeof(*rel); i++) {
		/* loc corresponds to P in the kernel space */
		loc = (void *)shdrs[shdrs[relsec].sh_info].sh_addr +
		      rel[i].r_offset;

		/* uloc corresponds P in user space */
		uloc_sec = shdrs[shdrs[relsec].sh_info].sh_addralign;
		uloc = (void *)uloc_sec + rel[i].r_offset;

		/* sym is the ELF symbol we're referring to */
		sym = symtab + GELF_R_SYM(rel[i].r_info);
		if (GELF_ST_TYPE(sym->st_info) == STT_SECTION &&
		    sym->st_shndx < uelf->info.hdr->e_shnum)
			sym_name = uelf->info.shstrtab +
				   shdrs[sym->st_shndx].sh_name;
		else
			sym_name = uelf->strtab + sym->st_name;

		/* val corresponds to (S + A) */
		val = (s64)(sym->st_value + rel[i].r_addend);
		log_debug(
			"upatch: reloc symbol, name=%s, k_addr=0x%lx, u_addr=0x%lx, "
			"r_offset=0x%lx, st_value=0x%lx, r_addend=0x%lx \n",
			sym_name, shdrs[shdrs[relsec].sh_info].sh_addr,
			uloc_sec, rel[i].r_offset, sym->st_value, rel[i].r_addend);

		/* Perform the static relocation. */
		switch (GELF_R_TYPE(rel[i].r_info)) {
        case R_RISCV_NONE:
        case R_RISCV_TPREL_ADD:
            break;

        case R_RISCV_64:
            *(unsigned long *)loc = val;
            break;

		/* seems no need to recalculate as it should confined in the same func */
        case R_RISCV_BRANCH:
            val -= (unsigned long)uloc;
			if ((signed)val >= 4096 || (signed)val < -4096)
				goto overflow;
            *(unsigned *)loc = set_btype_imm(*(unsigned *)loc, val);
            break;

        case R_RISCV_JAL:
            val -= (unsigned long)uloc;
			if ((signed)val >= (1<<20) || (signed)val < -(1<<20))
				goto overflow;
            *(unsigned *)loc = set_jtype_imm(*(unsigned *)loc, val);
            break;

        case R_RISCV_CALL:
        case R_RISCV_CALL_PLT: // in our jump table, must not overflow
            val -= (unsigned long)uloc;
            *(unsigned *)loc = set_utype_imm(*(unsigned *)loc, val);
            *(unsigned *)(loc + 4) = set_itype_imm(*(unsigned *)(loc + 4), val);
            break;

        case R_RISCV_GOT_HI20:
        case R_RISCV_TLS_GOT_HI20:
		case R_RISCV_TLS_GD_HI20:
        case R_RISCV_PCREL_HI20:
            val -= (unsigned long)uloc;	// fall through
        case R_RISCV_HI20:
        case R_RISCV_TPREL_HI20:
            if ((long)val != (long)(int)val)
				goto overflow;
            *(unsigned *)loc = set_utype_imm(*(unsigned *)loc, val);
            break;

        case R_RISCV_PCREL_LO12_I:
            val = find_pcrel_hi_value(rel + i, i, symtab, sym->st_value - uloc_sec);
            if (val == 0)
				goto overflow;
            val -= sym->st_value;	// fall through
        case R_RISCV_LO12_I:
        case R_RISCV_TPREL_LO12_I:
            *(unsigned *)loc = set_itype_imm(*(unsigned *)loc, val);
            break;

        case R_RISCV_PCREL_LO12_S:
            val = find_pcrel_hi_value(rel + i, i, symtab, sym->st_value - uloc_sec);
            if (val == 0)
                goto overflow;
            val -= sym->st_value;	// fall through
        case R_RISCV_LO12_S:
        case R_RISCV_TPREL_LO12_S:
            *(unsigned *)loc = set_stype_imm(*(unsigned *)loc, val);
            break;

        /* inner function label calculation, must not overflow */
        case R_RISCV_ADD8:
		    *(char *)loc += val;
			break;
        case R_RISCV_ADD16:
		    *(short *)loc += val;
			break;
        case R_RISCV_ADD32:
		    *(int *)loc += val;
			break;
        case R_RISCV_ADD64:
		    *(long *)loc += val;
			break;

        case R_RISCV_SUB8:
		    *(char *)loc -= val;
			break;
        case R_RISCV_SUB16:
		    *(short *)loc -= val;
			break;
        case R_RISCV_SUB32:
		    *(int *)loc -= val;
			break;
        case R_RISCV_SUB64:
		    *(long *)loc -= val;
			break;

        case R_RISCV_RVC_BRANCH:
            val -= (unsigned long)uloc;
            if ((signed)val >= 256 || (signed)val < -256)
				goto overflow;
            *(unsigned short *)loc = set_cbtype_imm(*(unsigned short *)loc, val);
			break;

        case R_RISCV_RVC_JUMP:
            val -= (unsigned long)uloc;
            if ((signed)val >= 2048 || (signed)val < -2048)
				goto overflow;
            *(unsigned short *)loc = set_cjtype_imm(*(unsigned short *)loc, val);
            break;

        case R_RISCV_RVC_LUI:
            if ((signed)val >= (1<<17) || (signed)val < -(1<<17) || (val & 0x3f000) == 0)
				goto overflow;
            *(unsigned short *)loc = set_citype_imm(*(unsigned short *)loc, val);
            break;

        case R_RISCV_SET8:
            *(char *)loc = val;
            break;
        case R_RISCV_SET16:
            *(short *)loc = val;
            break;
        case R_RISCV_32_PCREL:
        case R_RISCV_PLT32:
            val -= (unsigned long)uloc; // fall through
        case R_RISCV_32:
        case R_RISCV_SET32:
            if ((long)val != (long)(int)val)
				goto overflow;
            *(int *)loc = val;
            break;

        case R_RISCV_SUB6:
		    char w6 = (*(char *)loc - (char)val) & 0x3f;
            *(char *)loc = (*(char *)loc & 0xc0) | w6;
            break;
        case R_RISCV_SET6:
            *(char *)loc = (*(char *)loc & 0xc0) | (val & 0x3f);
            break;

		default:
			log_error("upatch: unsupported RELA relocation: %lu\n",
				  GELF_R_TYPE(rel[i].r_info));
			return -ENOEXEC;
		}
	}
	return 0;

overflow:
	log_error("upatch: overflow in relocation type %d val %lx\n",
		  (int)GELF_R_TYPE(rel[i].r_info), val);
	return -ENOEXEC;
}
