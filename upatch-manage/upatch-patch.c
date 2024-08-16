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

#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include <sys/mman.h>
#include <sys/time.h>

#include "log.h"
#include "upatch-common.h"
#include "upatch-patch.h"
#include "upatch-process.h"
#include "upatch-ptrace.h"
#include "upatch-relocation.h"
#include "upatch-resolve.h"
#include "upatch-stack-check.h"

#define GET_MICROSECONDS(a, b) \
	((a.tv_sec - b.tv_sec) * 1000000 + (a.tv_usec - b.tv_usec))

#ifndef ARCH_SHF_SMALL
#define ARCH_SHF_SMALL 0
#endif
#ifndef SHF_RO_AFTER_INIT
#define SHF_RO_AFTER_INIT 0x00200000
#endif

/* If this is set, the section belongs in the init part of the module */
#define BITS_PER_LONG sizeof(unsigned long) * 8

static GElf_Off calculate_load_address(struct running_elf *relf,
				       bool check_code)
{
	int i;
	GElf_Off min_addr = (unsigned long)-1;

	/* TODO: for ET_DYN, consider check PIE */
	if (relf->info.hdr->e_type != ET_EXEC &&
	    relf->info.hdr->e_type != ET_DYN) {
		log_error("invalid elf type, it should be ET_EXEC or ET_DYN\n");
		goto out;
	}

	for (i = 0; i < relf->info.hdr->e_phnum; ++i) {
		if (relf->phdrs[i].p_type != PT_LOAD)
			continue;
		if (!check_code ||
		    (check_code && (relf->phdrs[i].p_flags & PF_X)))
			min_addr = (min_addr > relf->phdrs[i].p_vaddr) ?
					   relf->phdrs[i].p_vaddr :
					   min_addr;
		// min_addr = min(min_addr, relf->phdrs[i].p_vaddr);
	}

out:
	return min_addr;
}

static unsigned long calculate_mem_load(struct object_file *obj)
{
	struct obj_vm_area *ovma;
	unsigned long load_addr = (unsigned long)-1;

	list_for_each_entry(ovma, &obj->vma, list) {
		if (ovma->inmem.prot & PROT_EXEC) {
			load_addr = (load_addr > ovma->inmem.start) ?
					    ovma->inmem.start :
					    load_addr;
		}
	}

	return load_addr;
}

static int rewrite_section_headers(struct upatch_elf *uelf)
{
	unsigned int i;

	/* Handle SHF_ALLOC in this part */

	/* This should always be true, but let's be sure. */
	uelf->info.shdrs[0].sh_addr = 0;
	uelf->info.shdrs[0].sh_addralign = 0;

	for (i = 1; i < uelf->info.hdr->e_shnum; i++) {
		GElf_Shdr *shdr = &uelf->info.shdrs[i];
		if (shdr->sh_type != SHT_NOBITS &&
		    uelf->info.patch_size < shdr->sh_offset + shdr->sh_size) {
			log_error("upatch len %lu truncated\n",
				  uelf->info.patch_size);
			return -ENOEXEC;
		}

		/* Mark all sections sh_addr with their address in the
		   temporary image. */
		shdr->sh_addr = (size_t)uelf->info.hdr + shdr->sh_offset;
		log_debug("section %s at 0x%lx\n",
			  uelf->info.shstrtab + shdr->sh_name, shdr->sh_addr);
	}

	return 0;
}

static unsigned long get_offset(unsigned long *size,
		       GElf_Shdr *sechdr)
{
	unsigned long ret;

	ret = ALIGN(*size, (unsigned long)(sechdr->sh_addralign ?: 1));
	*size = (unsigned long)ret + sechdr->sh_size;
	return ret;
}

static void layout_upatch_info(struct upatch_elf *uelf)
{
	GElf_Shdr *upatch_func = uelf->info.shdrs + uelf->index.upatch_funcs;
	unsigned long num = upatch_func->sh_size / sizeof(struct upatch_patch_func);

	uelf->core_layout.info_size = uelf->core_layout.size;
	uelf->core_layout.size += sizeof(struct upatch_info) +
				  num * sizeof(struct upatch_info_func);
	uelf->core_layout.size = PAGE_ALIGN(uelf->core_layout.size);
}

static void layout_jmptable(struct upatch_elf *uelf)
{
	uelf->jmp_cur_entry = 0;
	uelf->jmp_max_entry = JMP_TABLE_MAX_ENTRY;
	uelf->jmp_offs = ALIGN(uelf->core_layout.size, sizeof(unsigned long));
	uelf->core_layout.size =
		uelf->jmp_offs + uelf->jmp_max_entry * get_jmp_table_entry();
	uelf->core_layout.text_size = uelf->core_layout.size;
}

static void layout_sections(struct upatch_elf *uelf)
{
	static unsigned long const masks[][2] = {
		/* NOTE: all executable code must be the last section
		 * in this array; otherwise modify the text_size
		 * finder in the two loops below */
		{ SHF_EXECINSTR | SHF_ALLOC, ARCH_SHF_SMALL },
		{ SHF_ALLOC, SHF_WRITE | ARCH_SHF_SMALL },
		{ SHF_RO_AFTER_INIT | SHF_ALLOC, ARCH_SHF_SMALL },
		{ SHF_WRITE | SHF_ALLOC, ARCH_SHF_SMALL },
		{ ARCH_SHF_SMALL | SHF_ALLOC, 0 }
	};
	unsigned int m, i;

	for (i = 0; i < uelf->info.hdr->e_shnum; i++)
		uelf->info.shdrs[i].sh_entsize = ~0UL;

	log_debug("upatch section allocation order:\n");
	for (m = 0; m < ARRAY_SIZE(masks); ++m) {
		for (i = 0; i < uelf->info.hdr->e_shnum; ++i) {
			GElf_Shdr *s = &uelf->info.shdrs[i];
			const char *sname = uelf->info.shstrtab + s->sh_name;

			if ((s->sh_flags & masks[m][0]) != masks[m][0] ||
			    (s->sh_flags & masks[m][1]) ||
			    s->sh_entsize != ~0UL)
				continue;

			s->sh_entsize =
				get_offset(&uelf->core_layout.size, s);
			log_debug("\tm = %d; %s: sh_entsize: 0x%lx\n", m, sname,
				  s->sh_entsize);
		}
		switch (m) {
		case 0: /* executable */
			uelf->core_layout.size =
				PAGE_ALIGN(uelf->core_layout.size);
			uelf->core_layout.text_size = uelf->core_layout.size;
			break;
		case 1: /* RO: text and ro-data */
			uelf->core_layout.size =
				PAGE_ALIGN(uelf->core_layout.size);
			uelf->core_layout.ro_size = uelf->core_layout.size;
			break;
		case 2: /* RO after init */
			uelf->core_layout.size =
				PAGE_ALIGN(uelf->core_layout.size);
			uelf->core_layout.ro_after_init_size =
				uelf->core_layout.size;
			break;
		case 3: /* whole core */
			uelf->core_layout.size =
				PAGE_ALIGN(uelf->core_layout.size);
			break;
		default:
			break;
		}
	}
}

/* TODO: only included used symbol */
static bool is_upatch_symbol(void)
{
	return true;
}
/*
 * We only allocate and copy the strings needed by the parts of symtab
 * we keep.  This is simple, but has the effect of making multiple
 * copies of duplicates.  We could be more sophisticated, see
 * linux-kernel thread starting with
 * <73defb5e4bca04a6431392cc341112b1@localhost>.
 */
static void layout_symtab(struct upatch_elf *uelf)
{
	GElf_Shdr *symsect = uelf->info.shdrs + uelf->index.sym;
	GElf_Shdr *strsect = uelf->info.shdrs + uelf->index.str;
	/* TODO: only support same arch as kernel now */
	const GElf_Sym *src;
	unsigned long i, nsrc, ndst, strtab_size = 0;

	/* Put symbol section at end of init part of module. */
	symsect->sh_flags |= SHF_ALLOC;
	symsect->sh_entsize = get_offset(&uelf->core_layout.size, symsect);
	log_debug("\t%s\n", uelf->info.shstrtab + symsect->sh_name);

	src = (void *)uelf->info.hdr + symsect->sh_offset;
	nsrc = symsect->sh_size / sizeof(*src);

	/* Compute total space required for the symbols' strtab. */
	for (ndst = i = 0; i < nsrc; i++) {
		if (i == 0 || is_upatch_symbol()) {
			strtab_size +=
				strlen(&uelf->strtab[src[i].st_name]) + 1;
			ndst++;
		}
	}

	/* Append room for core symbols at end of core part. */
	uelf->symoffs =
		ALIGN(uelf->core_layout.size, symsect->sh_addralign ?: 1);
	uelf->stroffs = uelf->core_layout.size =
		uelf->symoffs + ndst * sizeof(GElf_Sym);
	uelf->core_layout.size += strtab_size;
	uelf->core_typeoffs = uelf->core_layout.size;
	uelf->core_layout.size += ndst * sizeof(char);
	uelf->core_layout.size = PAGE_ALIGN(uelf->core_layout.size);

	/* Put string table section at end of init part of module. */
	strsect->sh_flags |= SHF_ALLOC;
	strsect->sh_entsize = get_offset(&uelf->core_layout.size, strsect);
	uelf->core_layout.size = PAGE_ALIGN(uelf->core_layout.size);
	log_debug("\t%s\n", uelf->info.shstrtab + strsect->sh_name);
}

static void *upatch_alloc(struct object_file *obj, size_t sz)
{
	int ret;
	unsigned long addr;
	struct vm_hole *hole = NULL;

	addr = object_find_patch_region(obj, sz, &hole);
	if (!addr)
		return NULL;

	addr = upatch_mmap_remote(proc2pctx(obj->proc), addr, sz,
				  PROT_READ | PROT_WRITE | PROT_EXEC,
				  MAP_FIXED | MAP_PRIVATE | MAP_ANONYMOUS,
				  (unsigned long)-1, 0);
	if (addr == 0) {
		return NULL;
	}

	log_debug("Allocated 0x%lx bytes at 0x%lx of '%s'\n", sz, addr, obj->name);

	// log_debug("Marking this space as busy\n");
	ret = vm_hole_split(hole, addr, addr + sz);
	if (ret) {
		// TODO: clear
		log_error("Failed to split vm hole\n");
		return NULL;
	}

	return (void *)addr;
}

static void upatch_free(struct object_file *obj, void *base,
			     unsigned long size)
{
	log_debug("Free patch memory %p\n", base);
	if (upatch_munmap_remote(proc2pctx(obj->proc), (unsigned long)base, size)) {
		log_error("Failed to free patch memory %p\n", base);
	}
}

static int __alloc_memory(struct object_file *obj_file,
			  struct upatch_layout *layout)
{
	/* Do the allocs. */
	layout->base = upatch_alloc(obj_file, layout->size);
	if (!layout->base) {
		return -errno;
	}

	layout->kbase = malloc(layout->size);
	if (!layout->kbase) {
		upatch_free(obj_file, layout->base, layout->size);
		return -errno;
	}

	memset(layout->kbase, 0, layout->size);

	return 0;
}

static int alloc_memory(struct upatch_elf *uelf, struct object_file *obj)
{
	int i, ret;

	/* Do the allocs. */
	ret = __alloc_memory(obj, &uelf->core_layout);
	if (ret) {
		return ret;
	}

	/* Transfer each section which specifies SHF_ALLOC */
	log_debug("Final section addresses:\n");
	for (i = 0; i < uelf->info.hdr->e_shnum; i++) {
		void *kdest;
		void *dest;
		GElf_Shdr *shdr = &uelf->info.shdrs[i];

		if (!(shdr->sh_flags & SHF_ALLOC)) {
			continue;
		}

		kdest = uelf->core_layout.kbase + shdr->sh_entsize;
		dest = uelf->core_layout.base + shdr->sh_entsize;

		if (shdr->sh_type != SHT_NOBITS) {
			memcpy(kdest, (void *)shdr->sh_addr, shdr->sh_size);
		}

		shdr->sh_addr = (unsigned long)kdest;
		/* overuse this attr to record user address */
		shdr->sh_addralign = (unsigned long)dest;
		log_debug("\t0x%lx %s <- 0x%lx\n", (long)kdest,
			  uelf->info.shstrtab + shdr->sh_name, (long)dest);
	}

	return 0;
}

static int post_memory(struct upatch_elf *uelf, struct object_file *obj)
{
	int ret = 0;

	log_debug("Post kbase %lx(%lx) to base %lx\n",
		  (unsigned long)uelf->core_layout.kbase,
		  uelf->core_layout.size,
		  (unsigned long)uelf->core_layout.base);
	ret = upatch_process_mem_write(obj->proc, uelf->core_layout.kbase,
				       (unsigned long)uelf->core_layout.base,
				       uelf->core_layout.size);
	if (ret) {
		log_error("Failed to move kbase to base, ret=%d\n", ret);
		goto out;
	}

out:
	return ret;
}

static int complete_info(struct upatch_elf *uelf, struct object_file *obj, const char *uuid)
{
	int ret = 0;
	struct upatch_info *uinfo =
		(void *)uelf->core_layout.kbase + uelf->core_layout.info_size;
	struct upatch_patch_func *upatch_funcs_addr =
		(void *)uelf->info.shdrs[uelf->index.upatch_funcs].sh_addr;

	memcpy(uinfo, UPATCH_HEADER, strlen(UPATCH_HEADER));
	uinfo->size = uelf->core_layout.size - uelf->core_layout.info_size;
	uinfo->start = (unsigned long)uelf->core_layout.base;
	uinfo->end =
		(unsigned long)uelf->core_layout.base + uelf->core_layout.size;
	uinfo->changed_func_num =
		uelf->info.shdrs[uelf->index.upatch_funcs].sh_size /
		sizeof(struct upatch_patch_func);
	memcpy(uinfo->id, uuid, strlen(uuid));

	log_normal("Changed insn:\n");
	uinfo->funcs = (void *)uinfo + sizeof(*uinfo);
	for (unsigned int i = 0; i < uinfo->changed_func_num; ++i) {
		struct upatch_info_func *upatch_func = &uinfo->funcs[i];

		upatch_func->old_addr =
			upatch_funcs_addr[i].old_addr + uelf->relf->load_bias;
		upatch_func->old_size = upatch_funcs_addr[i].old_size;
		upatch_func->new_addr = upatch_funcs_addr[i].new_addr;
		upatch_func->new_size = upatch_funcs_addr[i].new_size;
		log_debug("\tstart\tend\n");
		log_debug("old:\t0x%lx\t0x%lx\n", upatch_func->old_addr,
			upatch_func->old_addr + upatch_func->old_size);
		log_debug("new:\t0x%lx\t0x%lx\n", upatch_func->new_addr,
			upatch_func->new_addr + upatch_func->new_size);
		ret = upatch_process_mem_read(obj->proc, upatch_func->old_addr,
					      &upatch_func->old_insn,
					      get_origin_insn_len());
		if (ret) {
			log_error("can't read origin insn at 0x%lx - %d\n",
				  upatch_func->old_addr, ret);
			goto out;
		}

		upatch_func->new_insn = get_new_insn();

		log_normal("\t0x%lx(0x%lx 0x%lx -> 0x%lx 0x%lx)\n", upatch_func->old_addr,
			  upatch_func->old_insn[0], upatch_func->old_insn[1],
			  upatch_func->new_insn, upatch_func->new_addr);
	}

out:
	return ret;
}

static int unapply_patch(struct object_file *obj,
			 struct upatch_info_func *funcs,
			 unsigned long changed_func_num)
{
	log_normal("Changed insn:\n");
	for (unsigned int i = 0; i < changed_func_num; ++i) {
		log_normal("\t0x%lx(0x%lx -> 0x%lx)\n", funcs[i].old_addr,
			  funcs[i].new_insn, funcs[i].old_insn[0]);

		int ret = upatch_process_mem_write(obj->proc, &funcs[i].old_insn,
			(unsigned long)funcs[i].old_addr, get_origin_insn_len());

		if (ret) {
			log_error("Failed to write old insn at 0x%lx, ret=%d\n",
				funcs[i].old_addr, ret);
			return ret;
		}
	}
	return 0;
}

static int apply_patch(struct upatch_elf *uelf, struct object_file *obj)
{
	int ret = 0;
	unsigned int i;
	struct upatch_info *uinfo =
		(void *)uelf->core_layout.kbase + uelf->core_layout.info_size;

	for (i = 0; i < uinfo->changed_func_num; ++i) {
		struct upatch_info_func *upatch_func = &uinfo->funcs[i];

		// write jumper insn to first 8 bytes
		ret = upatch_process_mem_write(obj->proc, &upatch_func->new_insn,
			(unsigned long)upatch_func->old_addr, get_upatch_insn_len());
		if (ret) {
			log_error(
				"Failed to ptrace upatch func at 0x%lx(0x%lx) - %d\n",
				upatch_func->old_addr, upatch_func->new_insn,
				ret);
			goto out;
		}
		// write 64bit new addr to second 8 bytes
		ret = upatch_process_mem_write(obj->proc, &upatch_func->new_addr,
			(unsigned long)upatch_func->old_addr + get_upatch_insn_len(),
			get_upatch_addr_len());
		if (ret) {
			log_error(
				"Failed to ptrace upatch func at 0x%lx(0x%lx) - %d\n",
				upatch_func->old_addr + get_upatch_insn_len(),
				upatch_func->new_addr,
				ret);
			goto out;
		}
	}

out:
	if (ret) {
		unapply_patch(obj, uinfo->funcs, uinfo->changed_func_num);
	}
	return ret;
}

static int upatch_mprotect(struct upatch_elf *uelf, struct object_file *obj)
{
	int ret;

	if (uelf->core_layout.text_size > 0) {
		ret = upatch_mprotect_remote(
			proc2pctx(obj->proc),
			(unsigned long)uelf->core_layout.base,
			uelf->core_layout.text_size, PROT_READ | PROT_EXEC);
		if (ret < 0) {
			log_error("Failed to change upatch text protection to r-x");
			return ret;
		}
	}

	if (uelf->core_layout.ro_size > uelf->core_layout.text_size) {
		ret = upatch_mprotect_remote(
			proc2pctx(obj->proc),
			(unsigned long)uelf->core_layout.base + uelf->core_layout.text_size,
			uelf->core_layout.ro_size - uelf->core_layout.text_size,
			PROT_READ);
		if (ret < 0) {
			log_error("Failed to change upatch ro protection to r--");
			return ret;
		}
	}

	if (uelf->core_layout.ro_after_init_size > uelf->core_layout.ro_size) {
		ret = upatch_mprotect_remote(
			proc2pctx(obj->proc),
			(unsigned long)uelf->core_layout.base + uelf->core_layout.ro_size,
			uelf->core_layout.ro_after_init_size - uelf->core_layout.ro_size,
			PROT_READ);
		if (ret < 0) {
			log_error("Failed to change upatch ro init protection to r--");
			return ret;
		}
	}

	if (uelf->core_layout.info_size >
	    uelf->core_layout.ro_after_init_size) {
		ret = upatch_mprotect_remote(
			proc2pctx(obj->proc),
			(unsigned long)uelf->core_layout.base + uelf->core_layout.ro_after_init_size,
			uelf->core_layout.info_size - uelf->core_layout.ro_after_init_size,
			PROT_READ | PROT_WRITE);
		if (ret < 0) {
			log_error("Failed to change upatch rw protection to rw-");
			return ret;
		}
	}

	if (uelf->core_layout.size > uelf->core_layout.info_size) {
		ret = upatch_mprotect_remote(
			proc2pctx(obj->proc),
			(unsigned long)uelf->core_layout.base + uelf->core_layout.info_size,
			uelf->core_layout.size - uelf->core_layout.info_size,
			PROT_READ);
		if (ret < 0) {
			log_error("Failed to change upatch info protection to r--");
			return ret;
		}
	}

	return 0;
}

static int upatch_apply_patches(struct upatch_process *proc,
				struct upatch_elf *uelf, const char *uuid)
{
	int ret = 0;
	struct object_file *obj = NULL;
	GElf_Off min_addr;
	bool found = false;

	list_for_each_entry(obj, &proc->objs, list) {
		if (obj->inode == uelf->relf->info.inode) {
			found = true;
			break;
		}
	}

	if (!found) {
		ret = -1;
		log_debug("Cannot find inode %lu in pid %d, file is not loaded\n",
			uelf->relf->info.inode, proc->pid);
		goto out;
	}

	min_addr = calculate_load_address(uelf->relf, true);
	uelf->relf->load_start = calculate_mem_load(obj);
	uelf->relf->load_bias = uelf->relf->load_start - min_addr;
	log_debug("load_bias = %lx\n", uelf->relf->load_bias);

	ret = rewrite_section_headers(uelf);
	if (ret)
		goto free;

	// Caculate upatch mem size
	layout_jmptable(uelf);
	layout_sections(uelf);
	layout_symtab(uelf);
	layout_upatch_info(uelf);

	log_debug("calculate core layout = %lx\n", uelf->core_layout.size);
	log_debug(
		"Core layout: text_size = %lx, ro_size = %lx, ro_after_init_size = "
		"%lx, info = %lx, size = %lx\n",
		uelf->core_layout.text_size, uelf->core_layout.ro_size,
		uelf->core_layout.ro_after_init_size,
		uelf->core_layout.info_size, uelf->core_layout.size);

	/*
	 * Map patch as close to the original code as possible.
	 * Otherwise we can't use 32-bit jumps.
	 */
	ret = alloc_memory(uelf, obj);
	if (ret) {
		log_error("Failed to alloc patch memory\n");
		goto free;
	}

	ret = upatch_mprotect(uelf, obj);
	if (ret) {
		log_error("Failed to set patch memory permission\n");
		goto free;
	}

	/* Fix up syms, so that st_value is a pointer to location. */
	ret = simplify_symbols(uelf, obj);
	if (ret) {
		goto free;
	}

	/* upatch new address will be updated */
	ret = apply_relocations(uelf);
	if (ret) {
		goto free;
	}

	/* upatch upatch info */
	ret = complete_info(uelf, obj, uuid);
	if (ret) {
		goto free;
	}

	struct upatch_info *uinfo =
		(void *)uelf->core_layout.kbase + uelf->core_layout.info_size;
	ret = upatch_stack_check(uinfo, proc, ACTIVE);
	if (ret) {
		goto free;
	}
	ret = post_memory(uelf, obj);
	if (ret) {
		goto free;
	}

	ret = apply_patch(uelf, obj);
	if (ret) {
		goto free;
	}

	ret = 0;
	goto out;

// TODO: clear
free:
	upatch_free(obj, uelf->core_layout.base, uelf->core_layout.size);
out:
	return ret;
}

static void upatch_time_tick(int pid) {
	static struct timeval start_tv;
	static struct timeval end_tv;

	if ((end_tv.tv_sec != 0) || (end_tv.tv_usec != 0)) {
		memset(&start_tv, 0, sizeof(struct timeval));
		memset(&end_tv, 0, sizeof(struct timeval));
	}

	if ((start_tv.tv_sec == 0) && (start_tv.tv_usec == 0)) {
		gettimeofday(&start_tv, NULL);
	} else {
		gettimeofday(&end_tv, NULL);
	}

	if ((start_tv.tv_sec == 0) || (start_tv.tv_usec == 0) ||
		(end_tv.tv_sec == 0) || (end_tv.tv_usec == 0)) {
		return;
	}

	long frozen_time = GET_MICROSECONDS(end_tv, start_tv);
	log_normal("Process %d frozen time is %ld microsecond(s)\n",
		pid, frozen_time);
}

int upatch_process_uuid_exist(struct upatch_process *proc, const char *uuid)
{
	struct object_file *obj;
	struct object_patch *patch;
	list_for_each_entry(obj, &proc->objs, list) {
		if (!obj->is_patch) {
			continue;
		}
		list_for_each_entry(patch, &obj->applied_patch, list) {
			if (strncmp(patch->uinfo->id, uuid, UPATCH_ID_LEN) == 0) {
				return -EEXIST;
			}
		}
	}
	return 0;
}

int process_patch(int pid, struct upatch_elf *uelf, struct running_elf *relf, const char *uuid, const char *binary_path)
{
	struct upatch_process proc;

	// 查看process的信息，pid: maps, mem, cmdline, exe
	int ret = upatch_process_init(&proc, pid);
	if (ret < 0) {
		log_error("Failed to init process\n");
		goto out;
	}

	printf("Patch '%s' to ", uuid);
	upatch_process_print_short(&proc);

	ret = upatch_process_mem_open(&proc, MEM_READ);
	if (ret < 0) {
		log_error("Failed to open process memory\n");
		goto out_free;
	}

	// use uprobe to interpose function. the program has been executed to the entry
	// point

	/*
	 * For each object file that we want to patch (either binary itself or
	 * shared library) we need its ELF structure to perform relocations.
	 * Because we know uniq BuildID of the object the section addresses
	 * stored in the patch are valid for the original object.
	 */
	// 解析process的mem-maps，获得各个块的内存映射以及phdr
	ret = upatch_process_map_object_files(&proc);
	if (ret < 0) {
		log_error("Failed to read process memory mapping\n");
		goto out_free;
	}
	ret = upatch_process_uuid_exist(&proc, uuid);
	if (ret != 0) {
		ret = 0;
		log_error("Patch '%s' already exists\n", uuid);
		goto out_free;
	}
	ret = binary_init(relf, binary_path);
    if (ret) {
        log_error("Failed to load binary\n");
        goto out_free;
    }

    uelf->relf = relf;
	upatch_time_tick(pid);

	/* Finally, attach to process */
	ret = upatch_process_attach(&proc);
	if (ret < 0) {
		log_error("Failed to attach process\n");
		goto out_free;
	}

	// TODO: 栈解析
	// 应用
	ret = upatch_apply_patches(&proc, uelf, uuid);
	if (ret < 0) {
		log_error("Failed to apply patch\n");
		goto out_free;
	}

out_free:
	upatch_process_detach(&proc);
	upatch_time_tick(pid);

	upatch_process_destroy(&proc);

out:
	return ret;
}

static int upatch_unapply_patches(struct upatch_process *proc, const char *uuid)
{
	int ret = 0;
	struct object_file *obj = NULL;
	struct object_patch *patch = NULL;
	bool found = false;

	// Traverse all mapped memory and find all upatch memory
	list_for_each_entry(obj, &proc->objs, list) {
		if (!obj->is_patch) {
			continue;
		}
		// For eatch patch, check it's id and do remove
		list_for_each_entry(patch, &obj->applied_patch, list) {
			if (strncmp(patch->uinfo->id, uuid, UPATCH_ID_LEN) != 0) {
				continue;
			}
			found = true;

			ret = upatch_stack_check(patch->uinfo, proc, DEACTIVE);
			if (ret) {
				goto out;
			}

			ret = unapply_patch(obj, patch->uinfo->funcs, patch->uinfo->changed_func_num);
			if (ret) {
				goto out;
			}

			log_debug("munmap upatch layout core:\n");
			upatch_free(obj,
				(void *)patch->uinfo->start,
				patch->uinfo->end - patch->uinfo->start
			);

			break;
		}
	}

	if (!found) {
		log_warn("Patch '%s' is not found\n", uuid);
		goto out;
	}

out:
	return ret;
}

int process_unpatch(int pid, const char *uuid)
{
	struct upatch_process proc;

	// TODO: check build id
	// TODO: 栈解析
	// 查看process的信息，pid: maps, mem, cmdline, exe
	int ret = upatch_process_init(&proc, pid);
	if (ret < 0) {
		log_error("Failed to init process\n");
		goto out;
	}

	printf("Unpatch '%s' from ", uuid);
	upatch_process_print_short(&proc);

	ret = upatch_process_mem_open(&proc, MEM_READ);
	if (ret < 0) {
		log_error("Failed to open process memory\n");
		goto out_free;
	}

	// use uprobe to interpose function. the program has been executed to the entry
	// point

	/*
	 * For each object file that we want to patch (either binary itself or
	 * shared library) we need its ELF structure to perform relocations.
	 * Because we know uniq BuildID of the object the section addresses
	 * stored in the patch are valid for the original object.
	 */
	// 解析process的mem-maps，获得各个块的内存映射以及phdr
	ret = upatch_process_map_object_files(&proc);
	if (ret < 0) {
		log_error("Failed to read process memory mapping\n");
		goto out_free;
	}

	upatch_time_tick(pid);

	/* Finally, attach to process */
	ret = upatch_process_attach(&proc);
	if (ret < 0) {
		log_error("Failed to attach process\n");
		goto out_free;
	}

	// 应用
	ret = upatch_unapply_patches(&proc, uuid);
	if (ret < 0) {
		log_error("Failed to remove patch\n");
		goto out_free;
	}

out_free:
	upatch_process_detach(&proc);
	upatch_time_tick(pid);

	upatch_process_destroy(&proc);

out:
	return ret;
}

static int upatch_info(struct upatch_process *proc)
{
	struct object_file *obj = NULL;
	struct object_patch *patch = NULL;
	bool found = false;

	list_for_each_entry(obj, &proc->objs, list) {
		if (obj->is_patch) {
			found = true;
			break;
		}
	}

	if (!found)
		return found;

	found = false;
	list_for_each_entry(patch, &obj->applied_patch, list) {
		// TODO: check upatch_info id
		found = true;
		break;
	}

	return found;
}

int process_info(int pid)
{
	int ret;
	struct upatch_process proc;
	char *status = "error";

	// TODO: check build id
	// TODO: 栈解析
	// 查看process的信息，pid: maps, mem, cmdline, exe
	ret = upatch_process_init(&proc, pid);
	if (ret < 0) {
		log_error("Failed to init process\n");
		goto out;
	}

	ret = upatch_process_mem_open(&proc, MEM_READ);
	if (ret < 0) {
		log_error("Failed to open process memory\n");
		goto out_free;
	}

	ret = upatch_process_map_object_files(&proc);
	if (ret < 0) {
		log_error("Failed to read process memory mapping\n");
		goto out_free;
	}

	ret = upatch_info(&proc);
	if (ret) {
		status = "actived";
	}
	else {
		status = "removed";
	}

	ret = 0;

out_free:
	upatch_process_destroy(&proc);

out:
	log_normal("%s\n", status);
	return ret;
}
