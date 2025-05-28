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

#include <gelf.h>
#include <asm/ptrace.h>

#include "log.h"
#include "upatch-ptrace.h"
#include "upatch-resolve.h"

/*
 * auipc   t6,0x0
 * ld      t6,16(t6) # addr
 * jr      t6
 * undefined
 */
#define RISCV64_JMP_TABLE_JUMP0 0x010fbf8300000f97
#define RISCV64_JMP_TABLE_JUMP1 0x000f8067

struct upatch_jmp_table_entry {
    unsigned long inst[2];
    unsigned long addr[2];
};

unsigned int get_jmp_table_entry()
{
    return sizeof(struct upatch_jmp_table_entry);
}

static unsigned long setup_jmp_table(struct upatch_elf *uelf,
                                     unsigned long jmp_addr,
                                     unsigned long origin_addr)
{
    struct upatch_jmp_table_entry *table =
        uelf->core_layout.kbase + uelf->jmp_offs;
    unsigned int index = uelf->jmp_cur_entry;
    if (index >= uelf->jmp_max_entry) {
        log_error("jmp table overflow\n");
        return 0;
    }

    table[index].inst[0] = RISCV64_JMP_TABLE_JUMP0;
    table[index].inst[1] = RISCV64_JMP_TABLE_JUMP1;
    table[index].addr[0] = jmp_addr;
    table[index].addr[1] = origin_addr;
    uelf->jmp_cur_entry++;
    return (unsigned long)(uelf->core_layout.base + uelf->jmp_offs +
                           index * sizeof(struct upatch_jmp_table_entry));
}

unsigned long setup_got_table(struct upatch_elf *uelf,
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

    table[index].inst[0] = jmp_addr;
    table[index].inst[1] = tls_addr;
    table[index].addr[0] = 0xffffffff;
    table[index].addr[1] = 0xffffffff;
    uelf->jmp_cur_entry++;
    return (unsigned long)(uelf->core_layout.base + uelf->jmp_offs +
                           index * sizeof(struct upatch_jmp_table_entry));
}

unsigned long insert_plt_table(struct upatch_elf *uelf, struct object_file *obj,
                               unsigned long r_type __attribute__((unused)), unsigned long addr)
{
    unsigned long jmp_addr = 0xffffffff;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (upatch_process_mem_read(obj->proc, addr, &jmp_addr,
                                sizeof(jmp_addr))) {
        log_error("copy address failed\n");
        goto out;
    }

    elf_addr = setup_jmp_table(uelf, jmp_addr, (unsigned long)addr);

    log_debug("0x%lx: jmp_addr=0x%lx, tls_addr=0x%lx\n", elf_addr,
              jmp_addr, tls_addr);

out:
    return elf_addr;
}

unsigned long insert_got_table(struct upatch_elf *uelf, struct object_file *obj,
                               unsigned long r_type, unsigned long addr)
{
    unsigned long jmp_addr = 0xffffffff;
    unsigned long tls_addr = 0xffffffff;
    unsigned long elf_addr = 0;

    if (upatch_process_mem_read(obj->proc, addr, &jmp_addr,
                                sizeof(jmp_addr))) {
        log_error("copy address failed\n");
        goto out;
    }

    /*
     * Addr with this type means the symbol is a dynamic TLS variable.
     * Addr points to a GOT entry(16 bytes) having type
     *
     * typedef struct {
     *		unsigned long int ti_module;
     *		unsigned long int ti_offset;
     * } tls_index;
     *
     * We also need copy ti_offset to our jump table.
     *
     * The corresponding symbol will associate with TLS_GD_HI20
     * relocation type, using this tls_index as argument to call
     * `void *__tls_get_addr (tls_index *ti)` to resolve the real address.
     */
    if (r_type == R_RISCV_TLS_DTPMOD64 &&
        upatch_process_mem_read(obj->proc, addr + sizeof(unsigned long),
                                &tls_addr, sizeof(tls_addr))) {
        log_error("copy address failed\n");
        goto out;
    }

    elf_addr = setup_got_table(uelf, jmp_addr, tls_addr);

    log_debug("0x%lx: jmp_addr=0x%lx, tls_addr=0x%lx\n", elf_addr,
              jmp_addr, tls_addr);

out:
    return elf_addr;
}
