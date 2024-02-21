// SPDX-License-Identifier: GPL-2.0
/*
 * upatch-manage
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

#include <gelf.h>

#include "upatch-ptrace.h"
#include "upatch-resolve.h"

#define X86_64_JUMP_TABLE_JMP 0x90900000000225ff /* jmp [rip+2]; nop; nop */

struct upatch_jmp_table_entry {
	unsigned long inst;
	unsigned long addr;
};

unsigned int get_jmp_table_entry()
{
	return sizeof(struct upatch_jmp_table_entry);
}

static unsigned long setup_jmp_table(struct upatch_elf *uelf,
				     unsigned long jmp_addr)
{
	struct upatch_jmp_table_entry *table =
		uelf->core_layout.kbase + uelf->jmp_offs;
	unsigned int index = uelf->jmp_cur_entry;
	if (index >= uelf->jmp_max_entry) {
		log_error("jmp table overflow\n");
		return 0;
	}

	table[index].inst = X86_64_JUMP_TABLE_JMP;
	table[index].addr = jmp_addr;
	uelf->jmp_cur_entry++;
	return (unsigned long)(uelf->core_layout.base + uelf->jmp_offs +
			       index * sizeof(struct upatch_jmp_table_entry));
}

/*
 * Jmp tabale records address and used call instruction to execute it.
 * So, we need 'Inst' and 'addr'
 * GOT only need record address and resolve it by [got_addr].
 * To simplify design, use same table for both jmp table and GOT.
 */
static unsigned long setup_got_table(struct upatch_elf *uelf,
				     unsigned long jmp_addr,
				     unsigned long tls_addr)
{
	struct upatch_jmp_table_entry *table =
		uelf->core_layout.kbase + uelf->jmp_offs;
	unsigned int index = uelf->jmp_cur_entry;
	if (index >= uelf->jmp_max_entry) {
		log_error("got table overflow\n");
		return 0;
	}

	table[index].inst = jmp_addr;
	table[index].addr = tls_addr;
	uelf->jmp_cur_entry++;
	return (unsigned long)(uelf->core_layout.base + uelf->jmp_offs +
			       index * sizeof(struct upatch_jmp_table_entry));
}

unsigned long insert_plt_table(struct upatch_elf *uelf, struct object_file *obj,
			       unsigned long r_type, unsigned long addr)
{
	unsigned long jmp_addr;
	unsigned long elf_addr = 0;

	if (upatch_process_mem_read(obj->proc, addr, &jmp_addr,
				    sizeof(jmp_addr))) {
		log_error("copy address failed\n");
		goto out;
	}

	elf_addr = setup_jmp_table(uelf, jmp_addr);

	log_debug("0x%lx: jmp_addr=0x%lx\n", elf_addr, jmp_addr);

out:
	return elf_addr;
}

unsigned long insert_got_table(struct upatch_elf *uelf, struct object_file *obj,
			       unsigned long r_type, unsigned long addr)
{
	unsigned long jmp_addr;
	unsigned long tls_addr = 0xffffffff;
	unsigned long elf_addr = 0;

	if (upatch_process_mem_read(obj->proc, addr, &jmp_addr,
				    sizeof(jmp_addr))) {
		log_error("copy address failed\n");
		goto out;
	}

	/*
	 * R_X86_64_TLSGD: allocate two contiguous entries in the GOT to hold a
	 * tls_index structure tls_index has two unsigned long, the first one is
	 * R_X86_64_DTPMOD64.
	 */
	if (r_type == R_X86_64_DTPMOD64 &&
	    upatch_process_mem_read(obj->proc, addr + sizeof(unsigned long),
				    &tls_addr, sizeof(tls_addr))) {
		log_error("copy address failed\n");
		goto out;
	}

	elf_addr = setup_got_table(uelf, jmp_addr, tls_addr);

	log_debug("0x%lx: jmp_addr=0x%lx\n", elf_addr, jmp_addr);

out:
	return elf_addr;
}