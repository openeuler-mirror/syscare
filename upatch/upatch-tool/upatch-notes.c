/*
 * upatch-notes.c
 *
 * Copyright (C) 2023 Zongwu Li <lizongwu@huawei.com>
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


#include <gelf.h>
#include <string.h>
#include <argp.h>
#include <fcntl.h>
#include <unistd.h>

#include "log.h"
#include "list.h"
#include "elf-common.h"
#include "upatch-manage.h"


enum loglevel loglevel = NORMAL;
char *logprefix;

struct arguments {
    char *output_elf;
    char *running_elf;
    bool debug;
};

static struct argp_option options[] = {
    {"debug", 'd', NULL, 0, "Show debug output"},
    {"running", 'r', "running", 0, "Running binary file"},
    {"output", 'o', "output", 0, "Output object"},
    {NULL}
};

static char program_doc[] =
    "upatch-notes -- generate the notes object";

static char args_doc[] = "-r elf_file -o output_file";

const char *argp_program_version = UPATCH_VERSION;

static error_t check_opt(struct argp_state *state)
{
    struct arguments *arguments = state->input;

    if (arguments->output_elf == NULL ||
        arguments->running_elf == NULL) {
            argp_usage(state);
            return ARGP_ERR_UNKNOWN;
    }
    return 0;
}

static error_t parse_opt(int key, char *arg, struct argp_state *state)
{
    struct arguments *arguments = state->input;

    switch (key)
    {
        case 'd':
            arguments->debug = true;
            break;
        case 'o':
            arguments->output_elf = arg;
            break;
        case 'r':
            arguments->running_elf = arg;
            break;
        case ARGP_KEY_ARG:
            break;
        case ARGP_KEY_END:
            return check_opt(state);
        default:
            return ARGP_ERR_UNKNOWN;
    }
    return 0;
}

static struct argp argp = {options, parse_opt, args_doc, program_doc};

static void print_strtab(char *buf, size_t size)
{
    size_t i;

    for (i = 0; i < size; i++) {
        if (buf[i] == 0)
            log_debug("\\0");
        else
            log_debug("%c", buf[i]);
    }
}

static void create_section_list(struct running_elf *relf)
{
    Elf_Data *data;
    struct section *sec;
    char* section_name;
    size_t shstrndx;
    Elf_Scn *scn = NULL;
    GElf_Shdr shdr;
    int index = 1;

    if (elf_getshdrstrndx(relf->elf, &shstrndx))
        ERROR("elf_getshdrstrndx with error %s", elf_errmsg(0));

    while ((scn = elf_nextscn(relf->elf, scn)) != NULL) {
        if (!gelf_getshdr(scn, &shdr))
            ERROR("gelf_getshdr with error %s", elf_errmsg(0));

        if (shdr.sh_type != SHT_NOTE && shdr.sh_type != SHT_STRTAB)
            continue;

        section_name = elf_strptr(relf->elf, shstrndx, shdr.sh_name);
        if (!section_name)
            ERROR("elf_strptr with error %s", elf_errmsg(0));

        // only need ".shstrtab" and SHT_NOTE section
        if (shdr.sh_type == SHT_STRTAB && strcmp(section_name, ".shstrtab"))
            continue;

        data = elf_getdata(scn, NULL);
        if (!data)
            ERROR("elf_getdata with error %s", elf_errmsg(0));

        ALLOC_LINK(sec, &relf->sections);

        sec->name = section_name;
        sec->data = data;
        sec->sh = shdr;
        sec->index = index;
        index ++;
    }
}

void relf_init(char *elf_name, struct running_elf *relf)
{
    relf->fd = open(elf_name, O_RDONLY);
    if (relf->fd == -1)
        ERROR("open with errno = %d", errno);

    relf->elf = elf_begin(relf->fd, ELF_C_READ, NULL);
    if (!relf->elf)
        ERROR("elf_begin with error %s", elf_errmsg(0));

    INIT_LIST_HEAD(&relf->sections);
    create_section_list(relf);
}

static void _upatch_create_shstrtab(struct list_head *sections)
{
    size_t size, offset, len;
    struct section *shstrtab, *sec;
    char *buf;

    shstrtab = find_section_by_name(sections, ".shstrtab");
    if (!shstrtab)
        ERROR("find_section_by_name failed.");

    /* determine size of string table */
    size = 1;
    list_for_each_entry(sec, sections, list)
        size += strlen(sec->name) + 1;

    buf = malloc(size);
    if (!buf)
        ERROR("malloc shstrtab failed.");
    memset(buf, 0, size);

    offset = 1;
    list_for_each_entry(sec, sections, list) {
        len = strlen(sec->name) + 1;
        sec->sh.sh_name = (unsigned int)offset;
        memcpy(buf + offset, sec->name, len);
        offset += len;
    }

    if (offset != size)
        ERROR("shstrtab size mismatch.");

    shstrtab->data->d_buf = buf;
    shstrtab->data->d_size = size;

    log_debug("shstrtab: ");
    print_strtab(buf, size);
    log_debug("\n");

    list_for_each_entry(sec, sections, list)
        log_debug("%s @ shstrtab offset %d\n", sec->name, sec->sh.sh_name);
}

static void _upatch_write_output_elf(struct list_head *sections, Elf *elf, char *outfile, mode_t mode)
{
    int fd;
    Elf *elfout;
    Elf_Scn *scn;
    Elf_Data *data;
    GElf_Ehdr eh, ehout;
    GElf_Shdr sh;
    struct section *sec, *shstrtab;

    fd = creat(outfile, mode);
    if (fd == -1)
        ERROR("creat failed.");

    elfout = elf_begin(fd, ELF_C_WRITE, NULL);
    if (!elfout)
        ERROR("elf_begin failed.");

    /* alloc ELF header */
    if (!gelf_newehdr(elfout, gelf_getclass(elf)))
        ERROR("gelf_newehdr failed.");
    if (!gelf_getehdr(elfout, &ehout))
        ERROR("gelf_getehdr elfout failed.");
    if (!gelf_getehdr(elf, &eh))
        ERROR("gelf_getehdr elf failed.");

    memset(&ehout, 0, sizeof(ehout));
    ehout.e_ident[EI_DATA] = eh.e_ident[EI_DATA];
    ehout.e_machine = eh.e_machine;
    ehout.e_type = eh.e_type;
    ehout.e_version = EV_CURRENT;

    shstrtab = find_section_by_name(sections, ".shstrtab");
    if (!shstrtab)
        ERROR("missing .shstrtab sections in write output elf");

    ehout.e_shstrndx = (unsigned short)shstrtab->index;

    /* add changed sections */
    list_for_each_entry(sec, sections, list) {
        scn = elf_newscn(elfout);
        if (!scn)
            ERROR("elf_newscn failed.");

        data = elf_newdata(scn);
        if (!data)
            ERROR("elf_newdata failed.");

        if (!elf_flagdata(data, ELF_C_SET, ELF_F_DIRTY))
            ERROR("elf_flagdata failed.");

        data->d_type = sec->data->d_type;
        data->d_buf = sec->data->d_buf;
        data->d_size = sec->data->d_size;

        if (!gelf_getshdr(scn, &sh))
            ERROR("gelf_getshdr in adding changed sections");

        sh = sec->sh;

        if (!gelf_update_shdr(scn, &sh))
            ERROR("gelf_update_shdr failed.");
    }

    if (!gelf_update_ehdr(elfout, &ehout))
        ERROR("gelf_update_ehdr failed.");

    if (elf_update(elfout, ELF_C_WRITE) < 0)
        ERROR("elf_update failed.");

    elf_end(elfout);
    close(fd);
}

void upatch_write_notes_elf(struct running_elf *relf, char *outfile, mode_t mode)
{
    int fd;
    GElf_Ehdr eh;

    _upatch_create_shstrtab(&relf->sections);

    // relf's type is DYN, can't be linked
    if (!gelf_getehdr(relf->elf, &eh))
        ERROR("gelf_getehdr elf failed.");

    eh.e_type = ET_REL;
    if (!gelf_update_ehdr(relf->elf, &eh))
        ERROR("gelf_update_ehdr failed.");

    _upatch_write_output_elf(&relf->sections, relf->elf, outfile, mode);
}

int relf_destroy(struct running_elf *relf)
{
    struct section *sec, *safesec;

    list_for_each_entry_safe(sec, safesec, &relf->sections, list) {
        memset(sec, 0, sizeof(*sec));
        free(sec);
    }
    INIT_LIST_HEAD(&relf->sections);

    elf_end(relf->elf);
    relf->elf = NULL;
    close(relf->fd);
    relf->fd = -1;

    return 0;
}

int main(int argc, char*argv[])
{
    struct arguments arguments;
    struct running_elf relf;

    memset(&arguments, 0, sizeof(arguments));
    argp_parse(&argp, argc, argv, 0, NULL, &arguments);

    if (arguments.debug)
        loglevel = DEBUG;

    if (elf_version(EV_CURRENT) ==  EV_NONE)
        ERROR("ELF library initialization failed");

    relf_init(arguments.running_elf, &relf);

    upatch_write_notes_elf(&relf, arguments.output_elf, 0664);

    relf_destroy(&relf);

    return 0;
}