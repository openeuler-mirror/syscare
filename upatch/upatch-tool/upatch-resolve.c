#include "upatch-resolve.h"

#include <stdlib.h>
#include <stddef.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>

#include "list.h"
#include "upatch-meta.h"
#include "upatch-elf.h"
#include "log.h"

static GElf_Off calculate_load_address(struct running_elf *relf,
		bool check_code)
{
	int i;
	GElf_Off min_addr = -1;

	/* TODO: for ET_DYN, consider check PIE */
	if (relf->info.hdr->e_type != ET_EXEC &&
			relf->info.hdr->e_type != ET_DYN) {
		log_warn("invalid elf type, it should be ET_EXEC or ET_DYN\n");
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

static int list_add_symbol(struct list_head *head, patch_symbols_t *sym)
{
	patch_symbols_t *newsym = (patch_symbols_t *)malloc(sizeof(patch_symbols_t));
	if (newsym == NULL)
		return ENOMEM;

	memset(newsym, 0, sizeof(patch_symbols_t));
	strncpy(newsym->name, sym->name, sizeof(newsym->name));
	newsym->offset = sym->offset;
	INIT_LIST_HEAD(&newsym->self);
	list_add(&newsym->self, head);
	return 0;
}

struct list_head* patch_symbols_resolve(const char *target_elf, const char *patch_file) {
	struct upatch_elf uelf;
	struct running_elf relf;
	GElf_Shdr *upatch_shdr = NULL;
	struct upatch_patch_func *upatch_funcs = NULL;
	GElf_Off min_addr; // binary base
	int num;
	struct list_head *head = malloc(sizeof(struct list_head));

	INIT_LIST_HEAD(head);

	int ret = upatch_init(&uelf, patch_file);
	if (ret < 0) {
		log_warn("upatch-resolve: upatch_init failed\n");
		goto out;
	}

	ret = binary_init(&relf, target_elf);
	if (ret < 0) {
		log_warn("upatch-resolve: binary_init failed %d \n", ret);
		goto out;
	}

	if (check_build_id(&uelf.info, &relf.info) == false) {
		log_error("upatch-resolve: Build id mismatched!\n");
		goto out;
	}

	uelf.relf = &relf;
	upatch_shdr = &uelf.info.shdrs[uelf.index.upatch_funcs];
	upatch_funcs = uelf.info.patch_buff + upatch_shdr->sh_offset;
	min_addr = calculate_load_address(uelf.relf, false);
	if (min_addr == (GElf_Off)-1) {
		goto out;
	}

	num = upatch_shdr->sh_size / sizeof(*upatch_funcs);

	log_debug("upatch-resolve: sh_size %lu, sizeof %lu \n", upatch_shdr->sh_size, sizeof(*upatch_funcs));
	log_debug("upatch-resolve: elf base addr is 0x%lx, num is %d\n", min_addr, num);

	for (int i = 0; i < num; i++) {
		patch_symbols_t *sym = malloc(sizeof(patch_symbols_t));
		sprintf(sym->name, "sym_%d", i);
		sym->offset = upatch_funcs[i].old_addr - min_addr;;
		log_debug("+upatch-resolve: sym->offset addr is 0x%lx\n", sym->offset);
		list_add_symbol(head, sym);
	}

	return head;
out:
	free(head);
	return NULL;
}

void patch_symbols_free(struct list_head *symbols) {
	patch_symbols_t *sym, *next;

	if (!symbols)
		return;
	list_for_each_entry_safe (sym, next, symbols, self) {
		list_del(&sym->self);
		free(sym);
	}
}
