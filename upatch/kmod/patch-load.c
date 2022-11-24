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

#include <asm/module.h>

#include "patch-uprobe.h"
#include "common.h"
#include "patch.h"

#define PLT_RELO_NAME ".rela.plt"

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

struct upatch_module *upatch_module_get(struct upatch_entity *entity, pid_t pid)
{
    struct upatch_module *um;
    mutex_lock(&entity->module_list_lock);
    um = __upatch_module_get(entity, pid);
    mutex_unlock(&entity->module_list_lock);
    return um;
}

struct upatch_module *upatch_module_new(pid_t pid)
{
    struct upatch_module *um;
    um = kzalloc(sizeof(struct upatch_module), GFP_KERNEL);
    if (!um)
        return NULL;

    um->pid = pid;
    um->real_state = UPATCH_STATE_ATTACHED;
    um->set_state = UPATCH_STATE_ATTACHED;
    INIT_LIST_HEAD(&um->list);
    return um;
}

static int __upatch_module_insert(struct upatch_entity *entity,
    struct upatch_module *um)
{
    if (!um)
        return -EINVAL;

    if (__upatch_module_get(entity, um->pid))
        return -EINVAL; // return error to free um

    list_add(&um->list, &entity->module_list);
    return 0;
}

int upatch_module_insert(struct upatch_entity *entity,
    struct upatch_module *um)
{
    int ret;
    pr_info("insert module in 0x%lx with pid %d \n", (unsigned long)um, um->pid);
    mutex_lock(&entity->module_list_lock);
    ret = __upatch_module_insert(entity, um);
    mutex_unlock(&entity->module_list_lock);
    return ret;
}

void upatch_module_remove(struct upatch_entity *entity,
    struct upatch_module *um)
{
    mutex_lock(&entity->module_list_lock);
    list_del(&um->list);
    kfree(um);
    mutex_unlock(&entity->module_list_lock);
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
                mod->core_layout.text_size = mod->core_layout.size;
                pr_info("text size is 0x%x \n", mod->core_layout.size);
                break;
            case 1: /* RO: text and ro-data */
                mod->core_layout.ro_size = mod->core_layout.size;
                pr_info("read only size is 0x%x \n", mod->core_layout.size);
                break;
            case 2: /* RO after init */
			    mod->core_layout.ro_after_init_size = mod->core_layout.size;
                pr_info("read after init size is 0x%x \n", mod->core_layout.size);
                break;
            case 4: /* whole core */
                pr_info("whole size is 0x%x \n", mod->core_layout.size);
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

    /* Put string table section at end of init part of module. */
    strsect->sh_flags |= SHF_ALLOC;
    strsect->sh_entsize = get_offset(mod, &mod->init_layout.size, strsect,
					 info->index.str) | INIT_OFFSET_MASK;
    pr_debug("\t%s\n", info->secstrings + strsect->sh_name);
}

static void layout_jmptable(struct upatch_module *mod, struct upatch_load_info *info)
{
    info->jmp_cur_entry = 0;
    info->jmp_max_entry = JMP_TABLE_MAX_ENTRY;
    info->jmp_offs = ALIGN(mod->core_layout.size, sizeof(unsigned long));
    mod->core_layout.size = info->jmp_offs
        + info->jmp_max_entry * sizeof(struct upatch_jmp_table_entry);
}

/* TODO: lock for mm */
unsigned long get_upatch_pole(unsigned long hint, unsigned long size)
{
    unsigned long range;
    unsigned search = hint;
    struct vm_area_struct *vma = find_vma(current->mm, search);
    while (vma) {
        search = vma->vm_end;
        range = vma->vm_next->vm_start - vma->vm_end;
        if (range > size)
            break;
        vma = vma->vm_next;
    }
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
    if (mem_addr != addr) {
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
    if (mod->init_layout.base)
        upatch_module_memfree(&mod->init_layout);
    mod->init_layout.base = NULL;
    if (mod->core_layout.base)
        upatch_module_memfree(&mod->core_layout);
    mod->core_layout.base = NULL;
}

static int upatch_module_alloc(struct upatch_load_info *info,
    struct upatch_module_layout *layout, unsigned long user_limit)
{
    layout->base = __upatch_module_alloc(info->running_elf.load_min, layout->size);
    if (!layout->base)
        return -ENOMEM;

    if ((unsigned long)layout->base - info->running_elf.load_min >= user_limit) {
        pr_err("out of range limit \n");
        __upatch_module_memfree(layout->base, layout->size);
        return -ENOMEM;
    }

    pr_info("upatch module at 0x%lx \n", (unsigned long)layout->base);

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
        pr_err("alloc upatch module memory failed \n");
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
	pr_info("final section addresses:\n");
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
		pr_info("\t0x%lx %s <- 0x%lx\n",
		    (long)dest, info->secstrings + shdr->sh_name, (long)kdest);
	}

    pr_info("move module finished \n");

    return 0;
}

static struct upatch_module *layout_and_allocate(struct upatch_load_info *info)
{
    struct upatch_module *mod;
    // unsigned int ndx;
    int err;

    err = check_modinfo();
    if (err)
        return ERR_PTR(err);

    layout_sections(info->mod, info);
    layout_symtab(info->mod, info);
    layout_jmptable(info->mod, info);

    err = move_module(info->mod, info);
    if (err)
        return ERR_PTR(err);

    /* TODO: update mod meta data info here */
    // mod = (void *)info->sechdrs[info->index.mod].sh_addr;
    mod = info->mod;
    return mod;
}

/* TODO: check status for in-list patches */
static int add_upatch_unformed_mod(struct upatch_module *mod)
{
    return 0;
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
    pr_info("sym is at 0x%lx \n", (unsigned long)mod->syms);
    mod->upatch_funs = section_objs(info, ".upatch.funcs",
				 sizeof(*mod->syms), &mod->num_upatch_funcs);
    pr_info("upatch_funs is at 0x%lx \n", (unsigned long)mod->upatch_funs);
    mod->strtab = section_addr(info, ".strtab");
    pr_info("strtab is at 0x%lx \n", (unsigned long)mod->strtab);
    return 0;
}

static unsigned long setup_jmp_table(struct upatch_load_info *info, unsigned long jmp_addr)
{
    struct upatch_jmp_table_entry *table =
        info->mod->core_layout.kbase + info->jmp_offs;
    unsigned int index = info->jmp_cur_entry;
    if (index >= info->jmp_max_entry) {
        pr_err("jmp table overflow \n");
        return 0;
    }

    table[index].inst = JMP_TABLE_JUMP;
    table[index].addr = jmp_addr;
    info->jmp_cur_entry ++;
    return (unsigned long)(info->mod->core_layout.base + info->jmp_offs
        + index * sizeof(struct upatch_jmp_table_entry));
}

static unsigned long
resolve_symbol(struct running_elf_info *elf_info, const char *name)
{
    unsigned int i;
    unsigned long elf_addr = 0;
    char *sym_name, *tmp;
    Elf_Shdr *sechdr;
    Elf_Sym *sym;
    Elf64_Rela *rela;

    sechdr = &elf_info->sechdrs[elf_info->index.sym];
    sym = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf_Sym); i++) {
        sym_name = elf_info->strtab + sym[i].st_name;
        /* TODO: do not care about version */
        tmp = strchr(sym_name, '@');
        if (tmp != NULL)
            *tmp = '\0';
        if (streql(sym_name, name) && sym[i].st_shndx != SHN_UNDEF) {
            pr_debug("found resolved undefined symbol %s at 0x%llx \n", name, sym[i].st_value);
            elf_addr = elf_info->load_bias + sym[i].st_value;
            goto out;
        }
    }

    /* TODO: is that necessary to support rel? */
    if (!elf_info->index.reloplt)
        goto out;

    /* Several possible solutions here:
     * 1. use symbol address from .dynsym, it works in limited situations
     * 2. use address from PLT/GOT, problems are:
     *      1) range limit(use jmp table?)
     *      2) only support existed symbols
     * 3. read symbol from library, combined with load_bias, calculate it directly
     *    and then worked with jmp table.
     *
     * Currently, we choose approach 2.
     *
     */
    /* .rela.plt is relocations for .dynsym */
    sechdr = &elf_info->sechdrs[elf_info->index.dynsym];
    sym = (void *)elf_info->hdr + sechdr->sh_offset;

    sechdr = &elf_info->sechdrs[elf_info->index.reloplt];
    rela = (void *)elf_info->hdr + sechdr->sh_offset;
    for (i = 0; i < sechdr->sh_size / sizeof(Elf64_Rela); i ++) {
        unsigned long r_sym = ELF64_R_SYM (rela[i].r_info);
        /* for executable file, r_offset is virtual address */
        void __user *tmp_addr = (void *)(elf_info->load_bias + rela[i].r_offset);
        unsigned long addr;

        if (copy_from_user((void *)&addr, tmp_addr, sizeof(unsigned long))) {
            pr_err("copy address failed \n");
            goto out;
        }

        sym_name = elf_info->dynstrtab + sym[r_sym].st_name;
        if (streql(sym_name, name)) {
            elf_addr = setup_jmp_table(elf_info->load_info, addr);
            pr_info("found unresolved plt.rela %s at 0x%llx -> 0x%lx <- 0x%lx (jmp)\n",
                sym_name, rela[i].r_offset, addr, elf_addr);
            goto out;
        }
    }

out:
    if (!elf_addr) {
        pr_err("unable to found valid symbol %s \n", name);
    }
    return elf_addr;
}

/* TODO: set timeout */
static inline unsigned long resolve_symbol_wait(struct upatch_module *mod,
    struct upatch_load_info *info, const char *name)
{
    return resolve_symbol(&info->running_elf, name);
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
            pr_info("absolute symbol %s \n", name);
            break;
        case SHN_UNDEF:
            elf_addr = resolve_symbol_wait(mod, info, name);
            if (!elf_addr)
                ret = -ENOEXEC;
            sym[i].st_value = elf_addr;
            pr_info("resolved symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        case SHN_LIVEPATCH:
            sym[i].st_value += info->running_elf.load_bias;
            pr_info("resolved livepatch symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        default:
            /* use real address to calculate secbase */
            secbase = info->sechdrs[sym[i].st_shndx].sh_addralign;
            sym[i].st_value += secbase;
            pr_info("normal symbol %s at 0x%lx \n",
                name, (unsigned long)sym[i].st_value);
            break;
        }
    }

    return ret;
}

/* TODO: arch releated */
int apply_relocate_add(Elf64_Shdr *sechdrs, const char *strtab,
		   unsigned int symindex, unsigned int relsec, struct upatch_module *me)
{
    unsigned int i;
    Elf64_Rela *rel = (void *)sechdrs[relsec].sh_addr;
    Elf64_Sym *sym;
    void *loc, *real_loc;
    u64 val;
    const char *name;

    pr_debug("Applying relocate section %u to %u\n",
	       relsec, sechdrs[relsec].sh_info);

    for (i = 0; i < sechdrs[relsec].sh_size / sizeof(*rel); i++) {
        /* This is where to make the change, calculate it from kernel address */
        loc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addr
			+ rel[i].r_offset;

        real_loc = (void *)sechdrs[sechdrs[relsec].sh_info].sh_addralign
			+ rel[i].r_offset;

		/* This is the symbol it is referring to.  Note that all
		   undefined symbols have been resolved. */
		sym = (Elf64_Sym *)sechdrs[symindex].sh_addr
			+ ELF64_R_SYM(rel[i].r_info);

        pr_debug("type %d st_value %Lx r_addend %Lx loc %Lx\n",
		       (int)ELF64_R_TYPE(rel[i].r_info),
		       sym->st_value, rel[i].r_addend, (u64)loc);

        val = sym->st_value + rel[i].r_addend;
        switch (ELF64_R_TYPE(rel[i].r_info)) {
        case R_X86_64_NONE:
			break;
        case R_X86_64_64:
			if (*(u64 *)loc != 0)
				goto invalid_relocation;
			memcpy(loc, &val, 8);
			break;
        case R_X86_64_32:
			if (*(u32 *)loc != 0)
				goto invalid_relocation;
			memcpy(loc, &val, 4);
			if (val != *(u32 *)loc
                && (ELF_ST_TYPE(sym->st_info) != STT_SECTION))
				goto overflow;
			break;
        case R_X86_64_32S:
			if (*(s32 *)loc != 0)
				goto invalid_relocation;
			memcpy(loc, &val, 4);
			if ((s64)val != *(s32 *)loc
                && (ELF_ST_TYPE(sym->st_info) != STT_SECTION))
				goto overflow;
			break;
        		case R_X86_64_PC32:
		case R_X86_64_PLT32:
			if (*(u32 *)loc != 0)
				goto invalid_relocation;
			val -= (u64)real_loc;
			memcpy(loc, &val, 4);
            break;
		case R_X86_64_PC64:
			if (*(u64 *)loc != 0)
				goto invalid_relocation;
			val -= (u64)real_loc;
			memcpy(loc, &val, 8);
			break;
		default:
			pr_err("Unknown rela relocation: %llu\n", ELF64_R_TYPE(rel[i].r_info));
			return -ENOEXEC;
        }
    }
    return 0;

invalid_relocation:
	pr_err("x86/modules: Skipping invalid relocation target, \
        existing value is nonzero for type %d, loc %p, name %s\n",
	    (int)ELF64_R_TYPE(rel[i].r_info), loc, name);
	return -ENOEXEC;

overflow:
	pr_err("overflow in relocation type %d name %s\n",
	       (int)ELF64_R_TYPE(rel[i].r_info), name);
	return -ENOEXEC;
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
            err = apply_relocate_add(info->sechdrs, info->strtab,
                info->index.sym, i, mod);
        }

        if (err < 0)
            break;
    }
    return err;
}

static int move_to_user(struct upatch_module_layout *layout)
{
    pr_info("mov content from 0x%lx to 0x%lx with 0x%x \n",
        (unsigned long)layout->kbase, (unsigned long)layout->base, layout->size);
    if (copy_to_user(layout->base, layout->kbase, layout->size))
        return -EPERM;
    return 0;
}

static int post_relocation(struct upatch_module *mod, struct upatch_load_info *info)
{
    int ret;

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
    elf_info->secstrings = (void *)elf_info->hdr
        + elf_info->sechdrs[elf_info->hdr->e_shstrndx].sh_offset;

    /* check section header */
    for (i = 1; i < elf_info->hdr->e_shnum; i++) {
        name = elf_info->secstrings + elf_info->sechdrs[i].sh_name;
        if (elf_info->sechdrs[i].sh_type == SHT_SYMTAB) {
            elf_info->index.sym = i;
            elf_info->index.symstr = elf_info->sechdrs[i].sh_link;
            elf_info->strtab = (char *)elf_info->hdr
                + elf_info->sechdrs[elf_info->index.symstr].sh_offset;
            pr_info("find index %d with str %d for symtab \n", i, elf_info->index.symstr);
        } else if (elf_info->sechdrs[i].sh_type == SHT_DYNSYM) {
            elf_info->index.dynsym = i;
            elf_info->index.dynsymstr = elf_info->sechdrs[i].sh_link;
            elf_info->dynstrtab = (char *)elf_info->hdr
                + elf_info->sechdrs[elf_info->index.dynsymstr].sh_offset;
            pr_info("find index %d with str %d for dynsym \n", i, elf_info->index.dynsymstr);
        } else if (elf_info->sechdrs[i].sh_type == SHT_DYNAMIC) {
            pr_info("find dynamic section %d, not use it now \n", i);
        } else if (streql(name, PLT_RELO_NAME)
            && elf_info->sechdrs[i].sh_type == SHT_RELA) {
            /* TODO: GOT is also need to be handled */
            elf_info->index.reloplt = i;
            pr_info("found %s with %d \n", PLT_RELO_NAME, i);
        }
    }

    if (elf_info->index.dynsym) {
        symsec = &elf_info->sechdrs[elf_info->index.dynsym];
        sym_addr = (void __user *)elf_info->load_bias
            + symsec->sh_addr;

        sym = (void *)elf_info->hdr + symsec->sh_offset;

        pr_info("dynamic symbol address at 0x%lx with 0x%llx \n",
            (unsigned long)sym_addr, symsec->sh_size);

        /* read dynamic symtab from memory and copy it to the binary_hdr */
        if (copy_from_user(sym, sym_addr, symsec->sh_size)) {
            pr_err("read dynsym failed \n");
            ret = -ENOMEM;
            goto out;
        }
    }

    if (!elf_info->index.sym && !elf_info->index.dynsym) {
        pr_err("no symtab/dynsym found \n");
        ret = -ENOEXEC;
        goto out;
    }

    ret = 0;
out:
    return ret;
}

static int complete_formation(struct upatch_module *mod, struct upatch_load_info *info)
{
    /* TODO: set memory previliage */
    mod->real_state = UPATCH_STATE_RESOLVED;
    return 0;
}

/* The main idea is from insmod */
int upatch_load(struct file *binary_file, struct file *patch_file,
    struct upatch_load_info *info)
{
    int err;
    loff_t offset;
    elf_addr_t min_addr;
    struct upatch_module *mod;

    pr_info("upatch_load works now \n");

    min_addr = calculate_load_address(binary_file, true);
    if (min_addr == -1) {
        pr_err("unable to obtain minimal execuatable address \n");
        err = -EINVAL;
        goto free_hdr;
    }

    info->running_elf.load_min = min_addr;
    pr_info("PT_X minimal address is 0x%lx \n", info->running_elf.load_min);

    /* TODO: any protect for start_code ? */
    info->running_elf.load_bias = current->mm->start_code - min_addr;

    pr_info("load bias for pid %d is 0x%lx \n",
        task_pid_nr(current), info->running_elf.load_bias);

    err = load_binary_syms(binary_file, &info->running_elf);
    if (err)
        goto free_hdr;

    info->running_elf.load_info = info;

    info->len = i_size_read(file_inode(patch_file));
    info->hdr = vmalloc(info->len);
    if (!info->hdr) {
        err = -ENOMEM;
        goto free_hdr;
    }

    offset = 0;
    /* read patch file into kernel memory */
    err = kernel_read(patch_file, info->hdr, info->len, &offset);
    if (err != info->len) {
        pr_err("read kernel failed - %d \n", err);
        err = -EINVAL;
        goto free_hdr;
    }

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

    err = add_upatch_unformed_mod(mod);
    if (err)
        goto free_module;

    /* after this step, everything should be in its final step */
    err = find_upatch_module_sections(mod, info);
    if (err)
        goto free_module;

    /* Fix up syms, so that st_value is a pointer to location. */
    err = simplify_symbols(mod, info);
    if (err < 0)
        goto free_module;

    err = apply_relocations(mod, info);
    if (err < 0)
        goto free_module;

    err = post_relocation(mod, info);
    if (err < 0)
        goto free_module;

    err = complete_formation(mod, info);
    if (err < 0)
        goto free_module;

    pr_info("patch load successfully \n");

    err = 0;

    goto free_hdr;
free_module:
    upatch_module_deallocate(mod);
free_hdr:
    load_info_clear(info);
    return err;
}