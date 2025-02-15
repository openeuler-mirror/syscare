// SPDX-License-Identifier: GPL-2.0
/*
 * elf-create.c
 *
 * Copyright (C) 2014 Seth Jennings <sjenning@redhat.com>
 * Copyright (C) 2013-2014 Josh Poimboeuf <jpoimboe@redhat.com>
 * Copyright (C) 2022 Longjun Luo <luolongjun@huawei.com>
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA,
 * 02110-1301, USA.
 */

#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>

#include "elf-common.h"
#include "elf-insn.h"
#include "elf-create.h"
#include "upatch-patch.h"
#include "upatch-dynrela.h"

/* create text and relocation sections */
static struct section *create_section_pair(struct upatch_elf *uelf, char *name,
    unsigned int entsize, unsigned int nr)
{
    char *relaname;
    struct section *sec;
    struct section *relasec;
    size_t size = strlen(name) + strlen(".rela") + 1;

    relaname = calloc(1, size);
    if (!relaname) {
        ERROR("relaname malloc failed.");
    }

    strcpy(relaname, ".rela");
    strcat(relaname, name);

    /* allocate text section resourcce */
    ALLOC_LINK(sec, &uelf->sections);
    sec->name = name;
    sec->data = calloc(1, sizeof(Elf_Data));
    if (!sec->data) {
        ERROR("section data malloc failed.");
    }
    sec->data_source = DATA_SOURCE_ALLOC;

    sec->data->d_buf = calloc(nr, entsize);
    if (!sec->data->d_buf) {
        ERROR("d_buf of section data malloc failed.");
    }
    sec->dbuf_source = DATA_SOURCE_ALLOC;

    sec->data->d_size = entsize * nr;
    sec->data->d_type = ELF_T_BYTE;

    /* set section header */
    sec->sh.sh_type = SHT_PROGBITS;
    sec->sh.sh_entsize = entsize;
    sec->sh.sh_addralign = 8;
    sec->sh.sh_flags = SHF_ALLOC;
    sec->sh.sh_size = entsize * nr;

    /* set relocation section */
    ALLOC_LINK(relasec, &uelf->sections);
    relasec->name = relaname;
    relasec->name_source = DATA_SOURCE_ALLOC;
    INIT_LIST_HEAD(&relasec->relas);

    /* buffers will be generated by upatch_rebuild_rela_section_data */
    relasec->data = calloc(1, sizeof(Elf_Data));
    if (!relasec->data) {
        ERROR("relasec data malloc failed.");
    }
    relasec->data_source = DATA_SOURCE_ALLOC;

    relasec->data->d_type = ELF_T_RELA;

    /* set relocation section header */
    relasec->sh.sh_type = SHT_RELA;
    relasec->sh.sh_entsize = sizeof(GElf_Rela);
    relasec->sh.sh_addralign = 8;

    relasec->base = sec;
    sec->rela = relasec;

    return sec;
}

/* create string section for elf */
void upatch_create_strings_elements(struct upatch_elf *uelf)
{
    struct section *sec;
    struct symbol *sym;

    /* create section header */
    ALLOC_LINK(sec, &uelf->sections);
    sec->name = ".upatch.strings";

    sec->data = calloc(1, sizeof(Elf_Data));
    if (!sec->data) {
        ERROR("section data malloc failed");
    }
    sec->data_source = DATA_SOURCE_ALLOC;

    sec->data->d_type = ELF_T_BYTE;

    /* set section header */
    sec->sh.sh_type = SHT_PROGBITS;
    sec->sh.sh_entsize = 1;
    sec->sh.sh_addralign = 1;
    sec->sh.sh_flags = SHF_ALLOC;

    /* create symbol */
    ALLOC_LINK(sym, &uelf->symbols);
    sym->sec = sec;
    sym->sym.st_info = GELF_ST_INFO(STB_LOCAL, STT_SECTION);
    sym->type = STT_SECTION;
    sym->bind = STB_LOCAL;
    sym->name = ".upatch.strings";
}

/* create upatch func info section */
void upatch_create_patches_sections(struct upatch_elf *uelf,
    struct running_elf *relf)
{
    struct symbol *sym;
    struct symbol *strsym;

    struct section *sec;
    struct section *relasec;

    struct upatch_patch_func *funcs;
    struct rela *rela;
    struct lookup_result symbol;

    unsigned int nr = 0;
    unsigned int index = 0;

    /* find changed func */
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->type != STT_FUNC || sym->status != CHANGED || sym->parent) {
            continue;
        }
        nr++;
    }

    /* create text/rela section pair */
    sec = create_section_pair(uelf, ".upatch.funcs", sizeof(*funcs),  nr);
    relasec = sec->rela;
    funcs = sec->data->d_buf;

    strsym = find_symbol_by_name(&uelf->symbols, ".upatch.strings");
    if (!strsym) {
        ERROR("Cannot find symbol '.upatch.strings'");
    }

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->type != STT_FUNC || sym->status != CHANGED || sym->parent) {
            continue;
        }
        if (!lookup_relf(relf, sym, &symbol)) {
            ERROR("Cannot find symbol '%s' in %s", sym->name, g_relf_name);
        }
        if (sym->bind == STB_LOCAL && symbol.global) {
            ERROR("Cannot find local symbol '%s' in symbol table.", sym->name);
        }
        log_debug("lookup for %s: symbol name %s sympos=%lu size=%lu.\n",
            sym->name, symbol.symbol->name, symbol.sympos, symbol.symbol->size);

        /* ATTENTION: kpatch convert global symbols to local symbols here. */
        funcs[index].old_addr = symbol.symbol->addr;
        funcs[index].old_size = symbol.symbol->size;
        funcs[index].new_size = sym->sym.st_size;
        funcs[index].sympos = symbol.sympos;

        log_debug("change func %s from 0x%lx.\n",
            sym->name, funcs[index].old_addr);

        /* Add a rela than will handle funcs[index].new_addr */
        ALLOC_LINK(rela, &relasec->relas);
        rela->sym = sym;
        rela->type = absolute_rela_type(uelf);
        rela->addend = 0;
        rela->offset = (unsigned int)(index * sizeof(*funcs));

        /* Add a rela than will handle funcs[index].name */
        ALLOC_LINK(rela, &relasec->relas);
        rela->sym = strsym;
        rela->type = absolute_rela_type(uelf);
        rela->addend = offset_of_string(&uelf->strings, sym->name);
        rela->offset = (unsigned int)(index * sizeof(*funcs) +
            offsetof(struct upatch_patch_func, name));

        index++;
    }

    if (index != nr) {
        ERROR("sanity check failed in funcs sections.\n");
    }
}

static bool need_dynrela(struct running_elf *relf, struct section *relasec,
    struct rela *rela)
{
    struct lookup_result symbol;

    if (is_debug_section(relasec) ||
        is_note_section(relasec)) {
        return false;
    }

    if (!lookup_relf(relf, rela->sym, &symbol)) {
        /* relocation is based on new symbol. */
        return false;
    }

    if (rela->sym->bind == STB_LOCAL) {
        if (symbol.global) {
            ERROR("No releated local symbol found.\n");
        }
        return true;
    }

    return false;
}

/*
 * This function is used to handle relocations which cannot be handled normally
 *
 * Situations:
 * 1. refer to old symbols
 *
 */
void upatch_create_intermediate_sections(struct upatch_elf *uelf,
    struct running_elf *relf)
{
    struct rela *rela, *rela_safe;
    struct section *relasec, *usym_sec, *urela_sec;
    struct upatch_symbol *usyms;
    struct upatch_relocation *urelas;
    struct symbol *strsym, *usym_sec_sym;
    unsigned int nr = 0, index = 0;

    list_for_each_entry(relasec, &uelf->sections, list) {
        if (!is_rela_section(relasec)) {
            continue;
        }
        /* no need to handle upatch meta section. */
        if (!strcmp(relasec->name, ".rela.upatch.funcs")) {
            continue;
        }
        list_for_each_entry(rela, &relasec->relas, list) {
            nr++;
            if (need_dynrela(relf, relasec, rela)) {
                rela->need_dynrela = 1;
            }
        }
    }

    urela_sec = create_section_pair(uelf, ".upatch.relocations",
        sizeof(*urelas), nr);
    urelas = urela_sec->data->d_buf;

    usym_sec = create_section_pair(uelf, ".upatch.symbols",
        sizeof(*usyms), nr);
    usyms = usym_sec->data->d_buf;

    ALLOC_LINK(usym_sec_sym, &uelf->symbols);
    usym_sec_sym->sec = usym_sec;
    usym_sec_sym->sym.st_info = GELF_ST_INFO(STB_LOCAL, STT_SECTION);
    usym_sec_sym->type = STT_SECTION;
    usym_sec_sym->bind = STB_LOCAL;
    usym_sec_sym->name = ".upatch.symbols";

    strsym = find_symbol_by_name(&uelf->symbols, ".upatch.strings");
    if (!strsym) {
        ERROR("can't find .upatch.strings symbol.\n");
    }

    list_for_each_entry(relasec, &uelf->sections, list) {
        if (!is_rela_section(relasec)) {
            continue;
        }
        if (!strcmp(relasec->name, ".rela.upatch.funcs") ||
            !strcmp(relasec->name, ".rela.upatch.relocations") ||
            !strcmp(relasec->name, ".rela.upatch.symbols")) {
            continue;
        }
        list_for_each_entry_safe(rela, rela_safe, &relasec->relas, list) {
            if (!rela->need_dynrela) {
                rela->sym->strip = SYMBOL_USED;
                continue;
            }
        }
    }

    log_debug("generate %d dynamic relocations.\n", index);

    /* set size to actual number of kyms/krelas */
    usym_sec->data->d_size = index * sizeof(struct upatch_symbol);
    usym_sec->sh.sh_size = usym_sec->data->d_size;

    urela_sec->data->d_size = index * sizeof(struct upatch_relocation);
    urela_sec->sh.sh_size = urela_sec->data->d_size;
}

void upatch_build_strings_section_data(struct upatch_elf *uelf)
{
    struct section *sec;
    struct string *string;
    size_t size;
    char *strtab;

    sec = find_section_by_name(&uelf->sections, ".upatch.strings");
    if (!sec) {
        ERROR("can't find strings section.");
    }

    size = 0;
    list_for_each_entry(string, &uelf->strings, list) {
        size += strlen(string->name) + 1;
    }

    /* allocate section resources */
    strtab = calloc(1, size);
    if (!strtab) {
        ERROR("strtab malloc failed.");
    }

    sec->data->d_buf = strtab;
    sec->data->d_size = size;
    sec->dbuf_source = DATA_SOURCE_ALLOC;

    /* populate strings section data */
    list_for_each_entry(string, &uelf->strings, list) {
        log_debug("add string %s.\n", string->name);
        strcpy(strtab, string->name);
        strtab += strlen(string->name) + 1;
    }
}

static void migrate_symbols(struct list_head *src,
    struct list_head *dst, bool (*select)(struct symbol *))
{
    struct symbol *sym, *sym_safe;

    list_for_each_entry_safe(sym, sym_safe, src, list) {
        if (select && !select(sym)) {
            continue;
        }
        list_del(&sym->list);
        list_add_tail(&sym->list, dst);
    }
}

/* include symbols by order */
void upatch_reorder_symbols(struct  upatch_elf *uelf)
{
    LIST_HEAD(symbols);

    /* migrate NULL symbol */
    migrate_symbols(&uelf->symbols, &symbols, is_null_sym);
    /* migrate LOCAL FILE symbol */
    migrate_symbols(&uelf->symbols, &symbols, is_file_sym);
    /* migrate LOCAL FUNC symbol */
    migrate_symbols(&uelf->symbols, &symbols, is_local_func_sym);
    /* migrate all other LOCAL symbol */
    migrate_symbols(&uelf->symbols, &symbols, is_local_sym);
    /* migrate all other (GLOBAL) symbol */
    migrate_symbols(&uelf->symbols, &symbols, NULL);

    /* use uelf->symbols to replace symbols */
    list_replace(&symbols, &uelf->symbols);
}

/* strip out symbols that is releated with dynrelas */
void upatch_strip_unneeded_syms(struct upatch_elf *uelf)
{
    struct symbol *sym, *sym_safe;

    list_for_each_entry_safe(sym, sym_safe, &uelf->symbols, list) {
        if (sym->strip == SYMBOL_STRIP) {
            list_del(&sym->list);
            free(sym);
        }
    }
}

void upatch_reindex_elements(struct upatch_elf *uelf)
{
    struct section *sec;
    struct symbol *sym;
    unsigned int index;

    index = 1;
    list_for_each_entry(sec, &uelf->sections, list) {
        sec->index = index;
        index++;
    }

    index = 0;
    list_for_each_entry(sym, &uelf->symbols, list) {
        sym->index = index;
        index++;
        if (sym->sec) {
            sym->sym.st_shndx = (unsigned short)sym->sec->index;
        } else if (sym->sym.st_shndx != SHN_ABS) {
            sym->sym.st_shndx = SHN_UNDEF;
        }
    }
}

static void rebuild_rela_section_data(struct section *sec)
{
    struct rela *rela;
    GElf_Rela *relas;
    size_t size;

    unsigned int nr = 0;
    unsigned int index = 0;

    list_for_each_entry(rela, &sec->relas, list) {
        nr++;
    }

    size = nr * sizeof(*relas);
    relas = calloc(1, size);
    if (!relas) {
        ERROR("relas malloc failed.");
    }

    sec->data->d_buf = relas;
    sec->data->d_size = size;
    sec->sh.sh_size = size;
    sec->dbuf_source = DATA_SOURCE_ALLOC;

    list_for_each_entry(rela, &sec->relas, list) {
        relas[index].r_offset = rela->offset;
        relas[index].r_addend = rela->addend;
        relas[index].r_info = GELF_R_INFO(rela->sym->index, rela->type);
        index++;
    }

    if (index != nr) {
        ERROR("size mismatch in rebuild rela section.");
    }
}

/* update index for relocations */
void upatch_rebuild_relocations(struct upatch_elf *uelf)
{
    struct section *relasec;
    struct section *symtab;

    symtab = find_section_by_name(&uelf->sections, ".symtab");
    if (!symtab) {
        ERROR("missing .symtab section in rebuild relocations.\n");
    }

    list_for_each_entry(relasec, &uelf->sections, list) {
        if (!is_rela_section(relasec)) {
            continue;
        }
        relasec->sh.sh_link = (Elf64_Word)symtab->index;
        relasec->sh.sh_info = (Elf64_Word)relasec->base->index;
        rebuild_rela_section_data(relasec);
    }
}

void upatch_check_relocations(void)
{
    log_debug("upatch_check_relocations does not work now.\n");
    return;
}

static void print_strtab(char *buf, size_t size)
{
    size_t i;

    for (i = 0; i < size; i++) {
        if (buf[i] == 0) {
            log_debug("\\0");
        } else {
            log_debug("%c", buf[i]);
        }
    }
}

void upatch_create_shstrtab(struct upatch_elf *uelf)
{
    struct section *shstrtab;
    struct section *sec;

    char *buf;
    size_t size;
    size_t offset;
    size_t len;

    shstrtab = find_section_by_name(&uelf->sections, ".shstrtab");
    if (!shstrtab) {
        ERROR("find_section_by_name failed.");
    }

    /* determine size of string table */
    size = 1;
    list_for_each_entry(sec, &uelf->sections, list) {
        size += strlen(sec->name) + 1;
    }

    buf = calloc(1, size);
    if (!buf) {
        ERROR("malloc shstrtab failed.");
    }

    offset = 1;
    list_for_each_entry(sec, &uelf->sections, list) {
        len = strlen(sec->name) + 1;
        sec->sh.sh_name = (unsigned int)offset;
        memcpy(buf + offset, sec->name, len);
        offset += len;
    }

    if (offset != size) {
        free(buf);
        ERROR("shstrtab size mismatch.");
    }

    shstrtab->data->d_buf = buf;
    shstrtab->data->d_size = size;
    shstrtab->dbuf_source = DATA_SOURCE_ALLOC;

    log_debug("shstrtab: ");
    print_strtab(buf, size);
    log_debug("\n");

    list_for_each_entry(sec, &uelf->sections, list) {
        log_debug("%s @ shstrtab offset %d\n", sec->name, sec->sh.sh_name);
    }
}

void upatch_create_strtab(struct upatch_elf *uelf)
{
    size_t size = 0;
    size_t offset = 0;
    size_t len = 0;

    struct section *strtab = find_section_by_name(&uelf->sections, ".strtab");
    if (!strtab) {
        ERROR("find section failed in create strtab.");
    }

    struct symbol *sym = NULL;
    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->type == STT_SECTION) {
            continue;
        }
        size += strlen(sym->name) + 1;
    }

    char *buf = calloc(1, size);
    if (!buf) {
        ERROR("malloc buf failed in create strtab");
    }

    list_for_each_entry(sym, &uelf->symbols, list) {
        if (sym->type == STT_SECTION) {
            sym->sym.st_name = 0;
            continue;
        }
        len = strlen(sym->name) + 1;
        sym->sym.st_name = (unsigned int)offset;
        memcpy(buf + offset, sym->name, len);
        offset += len;
    }

    if (offset != size) {
        free(buf);
        ERROR("shstrtab size mismatch.");
    }

    strtab->data->d_buf = buf;
    strtab->data->d_size = size;
    strtab->dbuf_source = DATA_SOURCE_ALLOC;

    log_debug("strtab: ");
    print_strtab(buf, size);
    log_debug("\n");

    list_for_each_entry(sym, &uelf->symbols, list) {
        log_debug("%s @ strtab offset %d\n", sym->name, sym->sym.st_name);
    }
}

void upatch_create_symtab(struct upatch_elf *uelf)
{
    struct section *symtab;
    struct section *strtab;
    struct symbol *sym;

    unsigned int nr = 0;
    unsigned int nr_local = 0;

    char *buf;
    size_t size;
    unsigned long offset = 0;

    symtab = find_section_by_name(&uelf->sections, ".symtab");
    if (!symtab) {
        ERROR("find_section_by_name failed.");
    }

    list_for_each_entry(sym, &uelf->symbols, list) {
        nr++;
    }

    size = nr * symtab->sh.sh_entsize;
    buf = calloc(1, size);
    if (!buf) {
        ERROR("malloc buf failed in create symtab.");
    }

    offset = 0;
    list_for_each_entry(sym, &uelf->symbols, list) {
        memcpy(buf + offset, &sym->sym, symtab->sh.sh_entsize);
        offset += symtab->sh.sh_entsize;
        if (is_local_sym(sym)) {
            nr_local++;
        }
    }

    symtab->data->d_buf = buf;
    symtab->data->d_size = size;
    symtab->dbuf_source = DATA_SOURCE_ALLOC;

    /* update symtab section header */
    strtab = find_section_by_name(&uelf->sections, ".strtab");
    if (!strtab) {
        ERROR("missing .strtab section in create symtab.");
    }

    symtab->sh.sh_link = (Elf64_Word)strtab->index;
    symtab->sh.sh_info = nr_local;
}

void upatch_write_output_elf(struct upatch_elf *uelf, Elf *elf,
    char *outfile, mode_t mode)
{
    int fd;

    Elf *elfout;

    Elf_Scn *scn;
    Elf_Data *data;

    GElf_Ehdr eh;
    GElf_Ehdr ehout;

    GElf_Shdr sh;

    struct section *sec;
    struct section *shstrtab;

    fd = creat(outfile, mode);
    if (fd == -1) {
        ERROR("creat failed.");
    }

    elfout = elf_begin(fd, ELF_C_WRITE, NULL);
    if (!elfout) {
        ERROR("elf_begin failed.");
    }

    /* alloc ELF header */
    if (!gelf_newehdr(elfout, gelf_getclass(elf))) {
        ERROR("gelf_newehdr failed.");
    }
    if (!gelf_getehdr(elfout, &ehout)) {
        ERROR("gelf_getehdr elfout failed.");
    }
    if (!gelf_getehdr(elf, &eh)) {
        ERROR("gelf_getehdr elf failed.");
    }

    memset(&ehout, 0, sizeof(ehout));
    ehout.e_ident[EI_DATA] = eh.e_ident[EI_DATA];
    ehout.e_machine = eh.e_machine;
    ehout.e_type = eh.e_type;
    ehout.e_version = EV_CURRENT;

    shstrtab = find_section_by_name(&uelf->sections, ".shstrtab");
    if (!shstrtab) {
        ERROR("missing .shstrtab sections in write output elf");
    }

    ehout.e_shstrndx = (unsigned short)shstrtab->index;

    /* add changed sections */
    list_for_each_entry(sec, &uelf->sections, list) {
        scn = elf_newscn(elfout);
        if (!scn) {
            ERROR("elf_newscn failed.");
        }

        data = elf_newdata(scn);
        if (!data) {
            ERROR("elf_newdata failed.");
        }

        if (!elf_flagdata(data, ELF_C_SET, ELF_F_DIRTY)) {
            ERROR("elf_flagdata failed.");
        }

        data->d_type = sec->data->d_type;
        data->d_buf = sec->data->d_buf;
        data->d_size = sec->data->d_size;

        if (!gelf_getshdr(scn, &sh)) {
            ERROR("gelf_getshdr in adding changed sections");
        }

        sh = sec->sh;

        if (!gelf_update_shdr(scn, &sh)) {
            ERROR("gelf_update_shdr failed.");
        }
    }

    if (!gelf_update_ehdr(elfout, &ehout)) {
        ERROR("gelf_update_ehdr failed.");
    }

    if (elf_update(elfout, ELF_C_WRITE) < 0) {
        ERROR("elf_update failed.");
    }

    elf_end(elfout);
    close(fd);
}
