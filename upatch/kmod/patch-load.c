// SPDX-License-Identifier: GPL-2.0
/*
 * Copyright (C) 2022 HUAWEI, Inc.
 *
 * Authors:
 *   Longjun Luo <luolongjuna@gmail.com>
 *
 */

#include <linux/fs.h>
#include <linux/elf.h>
#include <linux/list.h>
#include <linux/slab.h>
#include <linux/dcache.h>
#include <linux/file.h>
#include <linux/vmalloc.h>
#include <linux/kernel.h>
#include <linux/mm.h>
#include <linux/mman.h>
#include <linux/kprobes.h>

#include <asm/module.h>

#include "patch-uprobe.h"
#include "common.h"
#include "patch.h"
#include "arch/patch-load.h"
#include "kmod.h"

#define GOT_RELA_NAME ".rela.dyn"
#define PLT_RELA_NAME ".rela.plt"
#define TDATA_NAME ".tdata"
#define TBSS_NAME ".tbss"

#ifndef ARCH_SHF_SMALL
#define ARCH_SHF_SMALL 0
#endif

/* If this is set, the section belongs in the init part of the module */
#define INIT_OFFSET_MASK (1UL << (BITS_PER_LONG-1))

static int patch_header_check(struct upatch_load_info *info)
{
    if (info->len < sizeof(*(info->hdr)))
        return -ENOEXEC;

    if (memcmp(info->hdr->e_ident, ELFMAG, SELFMAG) != 0
        || info->hdr->e_type != ET_REL
        || !elf_check_arch(info->hdr)
        || info->hdr->e_shentsize != sizeof(Elf_Shdr))
        return -ENOEXEC;

    if (info->hdr->e_shoff >= info->len
	    || (info->hdr->e_shnum * sizeof(Elf_Shdr) >
		info->len - info->hdr->e_shoff))
		return -ENOEXEC;
    return 0;
}

static struct upatch_module *
__upatch_module_get(struct upatch_entity *entity, pid_t pid)
{
    struct upatch_module *um;
    list_for_each_entry(um, &entity->module_list, list) {
        if (um->pid == pid)
            return um;
    }
    return NULL;
}

struct upatch_module *__upatch_module_new(struct upatch_entity *entity, pid_t pid)
{
    struct upatch_module *um;
    um = kzalloc(sizeof(struct upatch_module), GFP_KERNEL);
    if (!um)
        return NULL;

    um->pid = pid;
    um->real_state = UPATCH_STATE_ATTACHED;
    um->real_patch = entity->set_patch;
    mutex_init(&um->module_status_lock);
    INIT_LIST_HEAD(&um->list);
    list_add(&um->list, &entity->module_list);
    return um;
}

struct upatch_module *upatch_module_get_or_create(struct upatch_entity *entity, pid_t pid)
{
    struct upatch_module *um;
    mutex_lock(&entity->entity_status_lock);
    um = __upatch_module_get(entity, pid);
    if (!um)
        um = __upatch_module_new(entity, pid);
    mutex_unlock(&entity->entity_status_lock);
    return um;
}

static int setup_load_info(struct upatch_load_info *info)
{
    unsigned int i;

    info->sechdrs = (void *)info->hdr + info->hdr->e_shoff;
    info->secstrings = (void *)info->hdr
        + info->sechdrs[info->hdr->e_shstrndx].sh_offset;

    for (i = 1; i < info->hdr->e_shnum; i++) {
        if (info->sechdrs[i].sh_type == SHT_SYMTAB) {
            info->index.sym = i;
            info->index.str = info->sechdrs[i].sh_link;
            info->strtab = (char *)info->hdr
                + info->sechdrs[info->index.str].sh_offset;
            break;
        }
    }

	if (!info->index.sym) {
		pr_warn("patch has no symbols (stripped?)\n");
		return -ENOEXEC;
	}

    return 0;
}

static int rewrite_section_headers(struct upatch_load_info *info)
{
    unsigned int i;

    /* Handle SHF_ALLOC in this part */

    /* This should always be true, but let's be sure. */
    info->sechdrs[0].sh_addr = 0;
    info->sechdrs[0].sh_addralign = 0;

    for (i = 1; i < info->hdr->e_shnum; i++) {
        Elf_Shdr *shdr = &info->sechdrs[i];
        if (shdr->sh_type != SHT_NOBITS
            && info->len < shdr->sh_offset + shdr->sh_size) {
            pr_err("upatch len %lu truncated\n", info->len);
            return -ENOEXEC;
        }

        /* Mark all sections sh_addr with their address in the
           temporary image. */
        shdr->sh_addr = (size_t)info->hdr + shdr->sh_offset;
        pr_debug("section %s at 0x%llx \n", info->secstrings + shdr->sh_name,
            shdr->sh_addr);
    }

    return 0;
}

/* TODO: check meta data in this func */
static int check_modinfo(void)
{
    return 0;
}

/* Additional bytes needed by arch in front of individual sections */
unsigned int __weak arch_mod_section_prepend(struct upatch_module *mod,
					     unsigned int section)
{
	/* default implementation just returns zero */
	return 0;
}

static long get_offset(struct upatch_module *mod, unsigned int *size,
		       Elf_Shdr *sechdr, unsigned int section)
{
	long ret;

	*size += arch_mod_section_prepend(mod, section);
	ret = ALIGN(*size, sechdr->sh_addralign ?: 1);
	*size = ret + sechdr->sh_size;
	return ret;
}

static void layout_jmptable(struct upatch_module *mod, struct upatch_load_info *info)
{
    info->jmp_cur_entry = 0;
    info->jmp_max_entry = JMP_TABLE_MAX_ENTRY;
    info->jmp_offs = ALIGN(mod->core_layout.size, sizeof(unsigned long));
    mod->core_layout.size = info->jmp_offs
        + info->jmp_max_entry * sizeof(struct upatch_jmp_table_entry);
}

static void layout_sections(struct upatch_module *mod, struct upatch_load_info *info)
{
	static unsigned long const masks[][2] = {
		/* NOTE: all executable code must be the first section
		 * in this array; otherwise modify the text_size
		 * finder in the two loops below */
		{ SHF_EXECINSTR | SHF_ALLOC, ARCH_SHF_SMALL },
		{ SHF_ALLOC, SHF_WRITE | ARCH_SHF_SMALL },
		{ SHF_RO_AFTER_INIT | SHF_ALLOC, ARCH_SHF_SMALL },
		{ SHF_WRITE | SHF_ALLOC, ARCH_SHF_SMALL },
		{ ARCH_SHF_SMALL | SHF_ALLOC, 0 }
	};
    unsigned int m, i;

    for (i = 0; i < info->hdr->e_shnum; i++)
		info->sechdrs[i].sh_entsize = ~0UL;

    pr_debug("upatch section allocation order: \n");
    for (m = 0; m < ARRAY_SIZE(masks); ++m) {
        for (i = 0; i < info->hdr->e_shnum; ++i) {
            Elf_Shdr *s = &info->sechdrs[i];
            const char *sname = info->secstrings + s->sh_name;

			if ((s->sh_flags & masks[m][0]) != masks[m][0]
			    || (s->sh_flags & masks[m][1])
			    || s->sh_entsize != ~0UL)
				continue;
            s->sh_entsize = get_offset(mod, &mod->core_layout.size, s, i);
            pr_debug("\t%s\n", sname);
        }
        switch (m) {
            case 0: /* executable */
                layout_jmptable(info->mod, info);
                mod->core_layout.size = PAGE_ALIGN(mod->core_layout.size);
                mod->core_layout.text_size = mod->core_layout.size;
                break;
            case 1: /* RO: text and ro-data */
                mod->core_layout.size = PAGE_ALIGN(mod->core_layout.size);
                mod->core_layout.ro_size = mod->core_layout.size;
                break;
            case 2: /* RO after init */
                mod->core_layout.size = PAGE_ALIGN(mod->core_layout.size);
                mod->core_layout.ro_after_init_size = mod->core_layout.size;
                break;
            case 4: /* whole core */
                mod->core_layout.size = PAGE_ALIGN(mod->core_layout.size);
                break;
        }
    }
}

/* TODO: only included used symbol */
static bool is_upatch_symbol(const Elf_Sym *src, const Elf_Shdr *sechdrs,
			   unsigned int shnum)
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
static void layout_symtab(struct upatch_module *mod, struct upatch_load_info *info)
{
    Elf_Shdr *symsect = info->sechdrs + info->index.sym;
    Elf_Shdr *strsect = info->sechdrs + info->index.str;
    /* TODO: only support same arch as kernel now */
    const Elf_Sym *src;
    unsigned int i, nsrc, ndst, strtab_size = 0;

    /* Put symbol section at end of init part of module. */
    symsect->sh_flags |= SHF_ALLOC;
    symsect->sh_entsize = get_offset(mod, &mod->init_layout.size, symsect,
					 info->index.sym) | INIT_OFFSET_MASK;
    pr_debug("\t%s\n", info->secstrings + symsect->sh_name);

    src = (void *)info->hdr + symsect->sh_offset;
    nsrc = symsect->sh_size / sizeof(*src);

    /* Compute total space required for the symbols' strtab. */
    for (ndst = i = 0; i < nsrc; i++) {
		if (i == 0 ||
		    is_upatch_symbol(src+i, info->sechdrs, info->hdr->e_shnum)) {
			strtab_size += strlen(&info->strtab[src[i].st_name])+1;
			ndst++;
		}
	}

    /* Append room for core symbols at end of core part. */
    info->symoffs = ALIGN(mod->core_layout.size, symsect->sh_addralign ?: 1);
    info->stroffs = mod->core_layout.size = info->symoffs + ndst * sizeof(Elf_Sym);
    mod->core_layout.size += strtab_size;
    info->core_typeoffs = mod->core_layout.size;
    mod->core_layout.size += ndst * sizeof(char);
    mod->core_layout.size = PAGE_ALIGN(mod->core_layout.size);

    /* Put string table section at end of init part of module. */
    strsect->sh_flags |= SHF_ALLOC;
    strsect->sh_entsize = get_offset(mod, &mod->init_layout.size, strsect,
					 info->index.str) | INIT_OFFSET_MASK;
    mod->init_layout.size = PAGE_ALIGN(mod->init_layout.size);
    pr_debug("\t%s\n", info->secstrings + strsect->sh_name);
}

/* TODO: lock for mm */
unsigned long get_upatch_pole(unsigned long search, unsigned long size)
{
    struct vm_area_struct *vma =
        find_vma_intersection(current->mm, search, search + size);
    while (vma) {
        search = vma->vm_end;
        vma = find_vma_intersection(current->mm, search, search + size);
    }
    pr_debug("find search address at 0x%lx \n", search);
    return search;
}

/* alloc memory in userspace */
static void __user *__upatch_module_alloc(unsigned long hint, unsigned long size)
{
    unsigned long mem_addr;
    unsigned long addr = get_upatch_pole(hint, size);
    if (!addr)
        return NULL;

    mem_addr = vm_mmap(NULL, addr, size,
        PROT_READ | PROT_WRITE | PROT_EXEC,
        MAP_ANONYMOUS | MAP_PRIVATE, 0);
    if (IS_ERR((void *)mem_addr)) {
        pr_err("mmap module memory faild with %ld \n", PTR_ERR((void *)mem_addr));
        return NULL;
    } else if (mem_addr != addr) {
        pr_err("find wrong place 0x%lx <- 0x%lx \n", mem_addr, addr);
        vm_munmap(mem_addr, size);
        return NULL;
    }

    return (void *)mem_addr;
}

int __upatch_module_memfree(void __user *addr, unsigned long size)
{
    return vm_munmap((unsigned long)addr, size);
}

int upatch_module_memfree(struct upatch_module_layout *layout)
{
    if (layout->kbase)
        vfree(layout->kbase);
    layout->kbase = NULL;
    return __upatch_module_memfree(layout->base, layout->size);
}

void upatch_module_deallocate(struct upatch_module *mod)
{
    // if (mod->init_layout.base)
    //     upatch_module_memfree(&mod->init_layout);
    mod->init_layout.base = NULL;
    // if (mod->core_layout.base)
    //     upatch_module_memfree(&mod->core_layout);
    mod->core_layout.base = NULL;
    mod->upatch_funs = NULL;
    mod->strtab = NULL;
    mod->syms = NULL;
}

static int upatch_module_alloc(struct upatch_load_info *info,
    struct upatch_module_layout *layout, unsigned long user_limit)
{
    /* find moudle location from code start place */
    unsigned long hint = info->running_elf.load_start;

    layout->base = __upatch_module_alloc(hint, layout->size);
    if (!layout->base)
        return -ENOMEM;

    if ((unsigned long)layout->base - hint >= user_limit) {
        pr_err("out of range limit \n");
        __upatch_module_memfree(layout->base, layout->size);
        return -ENOMEM;
    }

    pr_debug("upatch module at 0x%lx \n", (unsigned long)layout->base);

    layout->kbase = vmalloc(layout->size);
    if (!layout->kbase) {
        __upatch_module_memfree(layout->base, layout->size);
        return -ENOMEM;
    }

    memset(layout->kbase, 0, layout->size);

    return 0;
}

static void load_info_clear(struct upatch_load_info *info)
{
    if (info->mod->core_layout.kbase)
        vfree(info->mod->core_layout.kbase);
    info->mod->core_layout.kbase = NULL;
    if (info->mod->init_layout.kbase)
        vfree(info->mod->init_layout.kbase);
    info->mod->init_layout.kbase = NULL;
    if (info->running_elf.hdr)
        vfree(info->running_elf.hdr);
    info->running_elf.hdr = NULL;
    if (info->hdr)
        vfree(info->hdr);
    info->hdr = NULL;
}

static int move_module(struct upatch_module *mod, struct upatch_load_info *info)
{
	int i, ret;

    /* Do the allocs. */
    ret = upatch_module_alloc(info, &mod->core_layout, 0xffffffff);
    if (ret) {
        pr_err("alloc upatch module memory failed: %d \n", ret);
        return ret;
    }

    /* TODO: we do not use this section now */
    if (mod->init_layout.size) {
        ret = upatch_module_alloc(info, &mod->init_layout, 0xffffffff);
        if (ret) {
            upatch_module_memfree(&mod->core_layout);
            return -ENOMEM;
        }
    } else {
        mod->init_layout.base = NULL;
    }

    /* Transfer each section which specifies SHF_ALLOC */
	pr_debug("final section addresses:\n");
	for (i = 0; i < info->hdr->e_shnum; i++) {
		void __user *dest;
        void *kdest;
		Elf_Shdr *shdr = &info->sechdrs[i];

		if (!(shdr->sh_flags & SHF_ALLOC))
			continue;

		if (shdr->sh_entsize & INIT_OFFSET_MASK) {
            dest = mod->init_layout.base
				+ (shdr->sh_entsize & ~INIT_OFFSET_MASK);
            kdest = mod->init_layout.kbase
				+ (shdr->sh_entsize & ~INIT_OFFSET_MASK);
        } else {
            dest = mod->core_layout.base + shdr->sh_entsize;
            kdest = mod->core_layout.kbase + shdr->sh_entsize;
        }

		if (shdr->sh_type != SHT_NOBITS)
			memcpy(kdest, (void *)shdr->sh_addr, shdr->sh_size);
		/* Update sh_addr to point to copy in image. */
        shdr->sh_addr = (unsigned long)kdest;
        /* overuse this attr to record user address */
        shdr->sh_addralign = (unsigned long)dest;
		pr_debug("\t0x%lx %s <- 0x%lx\n",
		    (long)dest, info->secstrings + shdr->sh_name, (long)kdest);
	}

    return 0;
}

static struct upatch_module *layout_and_allocate(struct upatch_load_info *info)
{
    int err;

    err = check_modinfo();
    if (err)
        return ERR_PTR(err);

    layout_sections(info->mod, info);
    layout_symtab(info->mod, info);

    err = move_module(info->mod, info);
    if (err)
        return ERR_PTR(err);

    return info->mod;
}

static unsigned int find_sec(const struct upatch_load_info *info, const char *name)
{
	unsigned int i;

	for (i = 1; i < info->hdr->e_shnum; i++) {
		Elf_Shdr *shdr = &info->sechdrs[i];
		/* Alloc bit cleared means "ignore it." */
		if ((shdr->sh_flags & SHF_ALLOC)
		    && strcmp(info->secstrings + shdr->sh_name, name) == 0)
			return i;
	}
	return 0;
}

static void *section_addr(const struct upatch_load_info *info, const char *name)
{
	/* Section 0 has sh_addr 0. */
	return (void *)info->sechdrs[find_sec(info, name)].sh_addralign;
}

static void *section_objs(const struct upatch_load_info *info,
			  const char *name,
			  size_t object_size,
			  unsigned int *num)
{
	unsigned int sec = find_sec(info, name);

	/* Section 0 has sh_addr 0 and sh_size 0. */
	*num = info->sechdrs[sec].sh_size / object_size;
    /* use address from user space */
	return (void __user *)info->sechdrs[sec].sh_addralign;
}

/* handle special sections in this func */
static int find_upatch_module_sections(struct upatch_module *mod, struct upatch_load_info *info)
{
    mod->syms = section_objs(info, ".symtab",
				 sizeof(*mod->syms), &mod->num_syms);
    mod->upatch_funs = section_objs(info, ".upatch.funcs",
				 sizeof(*mod->syms), &mod->num_upatch_funcs);
    mod->strtab = section_addr(info, ".strtab");
    return 0;
}

static unsigned long
resolve_symbol(struct running_elf_info *elf_info, const char *name, Elf_Sym patch_sym)
{
    unsigned int i;
    unsigned long elf_addr = 0;
    char *sym_name, *tmp;
    Elf_Shdr *sechdr;
    Elf_Sym *sym;
    Elf64_Rela *rela;

    if (ELF_ST_TYPE(patch_sym.st_info) == STT_IFUNC &&
        (elf_info->hdr->e_ident[EI_OSABI] == ELFOSABI_GNU || elf_info->hdr->e_ident[EI_OSABI] == ELFOSABI_FREEBSD))
        goto out_plt;
    /* handle symbol table first, in most cases, symbol table does not exist */
    sechdr = &elf_info->sechdrs[elf_info->index.sym];
    sym = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf_Sym); i++) {
        sym_name = elf_info->strtab + sym[i].st_name;
        /* FIXME: handle version for external function */
        tmp = strchr(sym_name, '@');
        if (tmp != NULL)
            *tmp = '\0';
        if (streql(sym_name, name) && sym[i].st_shndx != SHN_UNDEF) {
            pr_debug("found resolved undefined symbol %s at 0x%llx \n", name, sym[i].st_value);
            elf_addr = elf_info->load_bias + sym[i].st_value;
            goto out;
        }
    }

    /*
     * Handle external symbol, several possible solutions here:
     * 1. use symbol address from .dynsym, but most of its address is still undefined
     * 2. use address from PLT/GOT, problems are:
     *    1) range limit(use jmp table?)
     *    2) only support existed symbols
     * 3. read symbol from library, combined with load_bias, calculate it directly
     *    and then worked with jmp table.
     *
     * Currently, we will try approach 1 and approach 2.
     * Approach 3 is more general, but difficulty to implement.
     */
out_plt:
    if (!elf_info->index.dynsym)
        goto out;

    sechdr = &elf_info->sechdrs[elf_info->index.dynsym];
    sym = (void *)elf_info->hdr + sechdr->sh_offset;

    /* handle external function */
    if (!elf_info->index.relaplt)
        goto out_got;

    sechdr = &elf_info->sechdrs[elf_info->index.relaplt];
    rela = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf64_Rela); i ++) {
        unsigned long r_sym = ELF64_R_SYM (rela[i].r_info);
        /* for executable file, r_offset is virtual address of PLT table */
        void __user *tmp_addr = (void *)(elf_info->load_bias + rela[i].r_offset);

        /* some rela don't have the symbol index, use the symbol's value and rela's addend to find the symbol.
         * for example, R_X86_64_IRELATIVE.
         */
        if (r_sym == 0) {
            if (rela[i].r_addend != patch_sym.st_value)
                continue;
            sprintf(sym_name, "%llx", rela[i].r_addend);
        }
        else {
            /* ATTENTION: should we consider the relocation type ? */
            sym_name = elf_info->dynstrtab + sym[r_sym].st_name;
            /* FIXME: consider version of the library */
            tmp = strchr(sym_name, '@');
            if (tmp != NULL)
                *tmp = '\0';

            if (!(streql(sym_name, name)
                && (ELF64_ST_TYPE(sym[r_sym].st_info) == STT_FUNC || ELF64_ST_TYPE(sym[r_sym].st_info) == STT_TLS)))
                continue;
        }

        elf_addr = insert_plt_table(elf_info->load_info, ELF64_R_TYPE(rela[i].r_info), tmp_addr);
        pr_debug("found unresolved plt.rela %s at 0x%llx -> 0x%lx\n",
            sym_name, rela[i].r_offset, elf_addr);
        goto out;
    }

out_got:
    /* handle external object, we need get it's address, used for R_X86_64_REX_GOTPCRELX */
    if (!elf_info->index.reladyn)
        goto out;

    sechdr = &elf_info->sechdrs[elf_info->index.reladyn];
    rela = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf64_Rela); i ++) {
        unsigned long r_sym = ELF64_R_SYM (rela[i].r_info);
        /* for executable file, r_offset is virtual address of GOT table */
        void __user *tmp_addr = (void *)(elf_info->load_bias + rela[i].r_offset);

        if (r_sym == 0) {
            if (rela[i].r_addend != patch_sym.st_value)
                continue;
            sprintf(sym_name, "%llx", rela[i].r_addend);
        }
        else {
            sym_name = elf_info->dynstrtab + sym[r_sym].st_name;
            /* TODO: don't care about its version here */
            tmp = strchr(sym_name, '@');
            if (tmp != NULL)
                *tmp = '\0';

            /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
            if (!streql(sym_name, name))
                continue;
        }

        elf_addr = insert_got_table(elf_info->load_info, ELF64_R_TYPE(rela[i].r_info), tmp_addr);
        pr_debug("found unresolved .got %s at 0x%lx \n", sym_name, elf_addr);
        goto out;
    }

    // get symbol address from .dynsym
    sechdr = &elf_info->sechdrs[elf_info->index.dynsym];
    sym = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf64_Sym); i ++) {
        void __user *tmp_addr;

        /* only need the st_value that is not 0 */
        if (sym[i].st_value == 0)
            continue;

        sym_name = elf_info->dynstrtab + sym[i].st_name;
        /* TODO: don't care about its version here */
        tmp = strchr(sym_name, '@');
        if (tmp != NULL)
            *tmp = '\0';

        /* function could also be part of the GOT with the type R_X86_64_GLOB_DAT */
        if (!streql(sym_name, name))
            continue;

        tmp_addr = (void *)(elf_info->load_bias + sym[i].st_value);
        elf_addr = insert_got_table(elf_info->load_info, 0, tmp_addr);
        pr_debug("found unresolved .got %s at 0x%lx \n", sym_name, elf_addr);
        goto out;
    }

out:
    if (!elf_addr) {
        pr_err("unable to found valid symbol %s \n", name);
    }
    return elf_addr;
}

/* TODO: set timeout */
static inline unsigned long resolve_symbol_wait(struct upatch_module *mod,
    struct upatch_load_info *info, const char *name, Elf_Sym patch_sym)
{
    return resolve_symbol(&info->running_elf, name, patch_sym);
}

static int simplify_symbols(struct upatch_module *mod, struct upatch_load_info *info)
{
	Elf_Shdr *symsec = &info->sechdrs[info->index.sym];
	Elf_Sym *sym = (void *)symsec->sh_addr;
	unsigned long secbase;
	unsigned int i;
	int ret = 0;
	unsigned long elf_addr;

    for (i = 1; i < symsec->sh_size / sizeof(Elf_Sym); i++) {
        const char *name;

        if (ELF_ST_TYPE(sym[i].st_info) == STT_SECTION
            && sym[i].st_shndx < info->hdr->e_shnum)
            name = info->secstrings + info->sechdrs[sym[i].st_shndx].sh_name;
        else
            name = info->strtab + sym[i].st_name;

        switch (sym[i].st_shndx) {
        case SHN_COMMON:
            pr_warn("unsupported Common symbol: %s\n", name);
            ret = -ENOEXEC;
            break;
        case SHN_ABS:
            break;
        case SHN_UNDEF:
            elf_addr = resolve_symbol_wait(mod, info, name, sym[i]);
            if (!elf_addr)
                ret = -ENOEXEC;
            sym[i].st_value = elf_addr;
            pr_debug("resolved symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        case SHN_LIVEPATCH:
            sym[i].st_value += info->running_elf.load_bias;
            pr_debug("resolved livepatch symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        default:
            /* use real address to calculate secbase */
            secbase = info->sechdrs[sym[i].st_shndx].sh_addralign;
            sym[i].st_value += secbase;
            pr_debug("normal symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        }
    }

    return ret;
}

static int apply_relocations(struct upatch_module *mod, struct upatch_load_info *info)
{
    unsigned int i;
    int err = 0;

    /* Now do relocations. */
    for (i = 1; i < info->hdr->e_shnum; i++) {
        unsigned int infosec = info->sechdrs[i].sh_info;
        const char *name = info->secstrings + info->sechdrs[i].sh_name;

		/* Not a valid relocation section? */
		if (infosec >= info->hdr->e_shnum)
			continue;

		/* Don't bother with non-allocated sections */
		if (!(info->sechdrs[infosec].sh_flags & SHF_ALLOC))
			continue;

        if (info->sechdrs[i].sh_type == SHT_REL) {
            pr_err("do rel relocations for %s \n", name);
            return -EPERM;
        } else if (info->sechdrs[i].sh_type == SHT_RELA) {
            pr_debug("do rela relocations for %s \n", name);
            err = apply_relocate_add(info, info->sechdrs, info->strtab,
                info->index.sym, i, mod);
        }

        if (err < 0)
            break;
    }
    return err;
}

static int move_to_user(struct upatch_module_layout *layout)
{
    pr_debug("mov content from 0x%lx to 0x%lx with 0x%x \n",
        (unsigned long)layout->kbase, (unsigned long)layout->base, layout->size);
    if (copy_to_user(layout->base, layout->kbase, layout->size))
        return -EPERM;
    return 0;
}

static int post_relocation(struct upatch_module *mod, struct upatch_load_info *info)
{
    int ret;

    mod->load_bias = info->running_elf.load_bias;
    ret = move_to_user(&mod->core_layout);
    if (ret)
        return ret;

    ret = move_to_user(&mod->init_layout);
    if (ret)
        return ret;

    return 0;
}

int load_binary_syms(struct file *binary_file, struct running_elf_info *elf_info)
{
    int ret;
    loff_t offset;
    unsigned int i;
    void __user *sym_addr;
    Elf_Shdr *symsec;
	Elf_Sym *sym;
    const char *name;

    elf_info->len = i_size_read(file_inode(binary_file));
    elf_info->hdr = vmalloc(elf_info->len);
    if (!elf_info->hdr) {
        ret = -ENOMEM;
        goto out;
    }

    /* TODO: no need to read the whole file */
    offset = 0;
    ret = kernel_read(binary_file, elf_info->hdr, elf_info->len, &offset);
    if (ret != elf_info->len) {
        pr_err("read binary file failed - %d \n", ret);
        ret = -ENOMEM;
        goto out;
    }

    elf_info->sechdrs = (void *)elf_info->hdr + elf_info->hdr->e_shoff;
    elf_info->prohdrs = (void *)elf_info->hdr + elf_info->hdr->e_phoff;
    elf_info->secstrings = (void *)elf_info->hdr
        + elf_info->sechdrs[elf_info->hdr->e_shstrndx].sh_offset;
    elf_info->tls_size = 0;

    /* check section header */
    for (i = 1; i < elf_info->hdr->e_shnum; i++) {
        name = elf_info->secstrings + elf_info->sechdrs[i].sh_name;
        if (elf_info->sechdrs[i].sh_type == SHT_SYMTAB) {
            elf_info->index.sym = i;
            elf_info->index.symstr = elf_info->sechdrs[i].sh_link;
            elf_info->strtab = (char *)elf_info->hdr
                + elf_info->sechdrs[elf_info->index.symstr].sh_offset;
        } else if (elf_info->sechdrs[i].sh_type == SHT_DYNSYM) {
            elf_info->index.dynsym = i;
            elf_info->index.dynsymstr = elf_info->sechdrs[i].sh_link;
            elf_info->dynstrtab = (char *)elf_info->hdr
                + elf_info->sechdrs[elf_info->index.dynsymstr].sh_offset;
        } else if (elf_info->sechdrs[i].sh_type == SHT_DYNAMIC) {
            /* Currently, we don't utilize it */
        } else if (streql(name, PLT_RELA_NAME)
            && elf_info->sechdrs[i].sh_type == SHT_RELA) {
            elf_info->index.relaplt = i;
            pr_debug("found %s with %d \n", PLT_RELA_NAME, i);
        } else if (streql(name, GOT_RELA_NAME)
            && elf_info->sechdrs[i].sh_type == SHT_RELA) {
            elf_info->index.reladyn = i;
            pr_debug("found %s with %d \n", GOT_RELA_NAME, i);
        }
    }

    for (i = 0; i < elf_info->hdr->e_phnum; i++) {
        if (elf_info->prohdrs[i].p_type == PT_TLS) {
            elf_info ->tls_size = elf_info->prohdrs[i].p_memsz;
            elf_info ->tls_align = elf_info->prohdrs[i].p_align;
            break;
        }
    }

    if (elf_info->index.dynsym) {
        symsec = &elf_info->sechdrs[elf_info->index.dynsym];
        sym_addr = (void __user *)elf_info->load_bias
            + symsec->sh_addr;

        sym = (void *)elf_info->hdr + symsec->sh_offset;

        pr_debug("dynamic symbol address at 0x%lx with 0x%llx \n",
            (unsigned long)sym_addr, symsec->sh_size);

        /* read dynamic symtab from memory and copy it to the binary_hdr */
        if (copy_from_user(sym, sym_addr, symsec->sh_size)) {
            pr_err("read dynsym failed \n");
            ret = -ENOMEM;
            goto out;
        }
    }

    /* TODO: it is possible that no symbol resolve is needed */
    if (!elf_info->index.sym && !elf_info->index.dynsym) {
        pr_err("no symtab/dynsym found \n");
        ret = -ENOEXEC;
        goto out;
    }

    ret = 0;
out:
    return ret;
}

/* works for 64-bit architecture */
static long (*orig_mprotect) (const struct pt_regs *regs);

static long krun_mprotect(unsigned long start, size_t len, unsigned long prot)
{
    struct pt_regs regs;
    setup_parameters(&regs, start, len, prot);
    return orig_mprotect(&regs);
}

static void frob_text(const struct upatch_module_layout *layout)
{
    krun_mprotect((unsigned long)layout->base, layout->text_size, PROT_READ | PROT_EXEC);
}

static void frob_rodata(const struct upatch_module_layout *layout)
{
    krun_mprotect((unsigned long)layout->base + layout->text_size, layout->ro_size - layout->text_size, PROT_READ);
}

static void frob_writable_data(const struct upatch_module_layout *layout)
{
    krun_mprotect((unsigned long)layout->base + layout->ro_after_init_size, layout->size - layout->ro_after_init_size, PROT_READ | PROT_WRITE);
}

static void set_memory_previliage(struct upatch_module *mod)
{
    orig_mprotect = (void *)sys_call_table_p[__NR_mprotect];
    frob_text(&mod->core_layout);
    frob_rodata(&mod->core_layout);
    frob_writable_data(&mod->core_layout);
    frob_text(&mod->init_layout);
    frob_rodata(&mod->init_layout);
    frob_writable_data(&mod->init_layout);
}

static int complete_formation(struct upatch_module *mod, struct inode *patch)
{
    set_memory_previliage(mod);
    mod->real_state = UPATCH_STATE_RESOLVED;
    mod->real_patch = patch;
    return 0;
}

/* The main idea is from insmod */
int upatch_load(struct file *binary_file, struct inode *set_patch,
    struct patch_entity *patch_entity, struct upatch_load_info *info)
{
    int err;
    struct upatch_module *mod;

    if (patch_entity == NULL) {
        pr_err("invalid patch entity \n");
        err = -EINVAL;
        goto free_hdr;
    }

    err = load_binary_syms(binary_file, &info->running_elf);
    if (err)
        goto free_hdr;

    info->running_elf.load_info = info;

    info->len = patch_entity->patch_size;
    info->hdr = vmalloc(info->len);
    if (!info->hdr) {
        err = -ENOMEM;
        goto free_hdr;
    }

    /* read patch file into kernel memory */
    memcpy(info->hdr, patch_entity->patch_buff, info->len);

    err = patch_header_check(info);
    if (err) {
        pr_err("upatch has invalid ELF header");
        goto free_hdr;
    }

    err = setup_load_info(info);
    if (err)
        goto free_hdr;

    /* update section address */
    err = rewrite_section_headers(info);
    if (err)
        goto free_hdr;

    mod = layout_and_allocate(info);
    if (IS_ERR(mod)) {
        err = PTR_ERR(mod);
        goto free_hdr;
    }

    /* after this step, everything should be in its final step */
    err = find_upatch_module_sections(mod, info);
    if (err)
        goto free_module;

    /* Fix up syms, so that st_value is a pointer to location. */
    err = simplify_symbols(mod, info);
    if (err < 0)
        goto free_module;

    /* upatch new address will be updated */
    err = apply_relocations(mod, info);
    if (err < 0)
        goto free_module;

    err = post_relocation(mod, info);
    if (err < 0)
        goto free_module;

    err = complete_formation(mod, set_patch);
    if (err < 0)
        goto free_module;

    pr_debug("patch load successfully \n");

    err = 0;

    goto free_hdr;
free_module:
    upatch_module_deallocate(mod);
free_hdr:
    load_info_clear(info);
    return err;
}
