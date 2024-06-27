// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::Path;

use anyhow::Result;
use log::trace;

use crate::elf::{self, HeaderRead, HeaderWrite, SymbolRead, SymbolWrite};

pub fn resolve_upatch<P, Q>(patch: Q, debuginfo: P) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut patch_elf = elf::write::Elf::parse(patch)?;
    let debuginfo_elf = elf::read::Elf::parse(debuginfo)?;

    let debuginfo_e_ident = debuginfo_elf.header()?.get_e_ident();
    let debuginfo_e_type = debuginfo_elf.header()?.get_e_type();
    let ei_osabi = elf::elf_ei_osabi(debuginfo_e_ident);

    patch_elf.header()?.set_e_ident(debuginfo_e_ident);

    let debuginfo_syms = &mut debuginfo_elf.symbols()?;

    for mut symbol in &mut patch_elf.symbols()? {
        /* No need to handle section symbol */
        let sym_st_info = symbol.get_st_info();
        if elf::elf_st_type(sym_st_info) == elf::STT_SECTION {
            continue;
        }

        let sym_other = symbol.get_st_other();
        if sym_other & elf::SYM_OTHER != 0 {
            // TODO: we can delete these symbol's section here.
            symbol.set_st_other(sym_other & !elf::SYM_OTHER);
            match symbol.get_st_value() {
                0 => symbol.set_st_shndx(elf::SHN_UNDEF),
                _ => symbol.set_st_shndx(elf::SHN_LIVEPATCH),
            };
        } else if symbol.get_st_shndx() == elf::SHN_UNDEF {
            if elf::elf_st_bind(sym_st_info) == elf::STB_LOCAL {
                /* only partly resolved undefined symbol could have st_value */
                if symbol.get_st_value() != 0 {
                    symbol.set_st_shndx(elf::SHN_LIVEPATCH);
                }
            } else {
                __partial_resolve_patch(&mut symbol, debuginfo_syms, ei_osabi)?;
            }
        } else { /* do nothing */
        }

        /*
         * In a shared library with position-independent code (PIC) (no pie),
         * Such code accesses all constant addresses through a global offset table (GOT).
         * TODO: consider check PIE
         */
        if debuginfo_e_type == elf::ET_DYN
            && elf::elf_st_bind(sym_st_info) == elf::STB_GLOBAL
            && elf::elf_st_type(sym_st_info) == elf::STT_OBJECT
            && symbol.get_st_shndx() == elf::SHN_LIVEPATCH
        {
            symbol.set_st_shndx(elf::SHN_UNDEF);
        }
    }
    Ok(())
}

fn __partial_resolve_patch(
    symbol: &mut elf::write::SymbolHeader,
    debuginfo_syms: &mut elf::read::SymbolHeaderTable,
    ei_osabi: u8,
) -> Result<()> {
    debuginfo_syms.reset(0);

    for debuginfo_sym in debuginfo_syms {
        /* No need to handle section symbol */
        let sym_info = debuginfo_sym.get_st_info();
        if elf::elf_st_type(sym_info) == elf::STT_SECTION {
            continue;
        }

        let debuginfo_name = debuginfo_sym.get_st_name();
        if elf::elf_st_bind(sym_info).ne(&elf::elf_st_bind(symbol.get_st_info()))
            || debuginfo_name.ne(symbol.get_st_name())
        {
            continue;
        }

        /* leave it to be handled in running time */
        if debuginfo_sym.get_st_shndx() == elf::SHN_UNDEF {
            continue;
        }

        // symbol type is STT_IFUNC, need search st_value in .plt table in upatch.
        let is_ifunc = (ei_osabi.eq(&elf::ELFOSABI_GNU) || ei_osabi.eq(&elf::ELFOSABI_FREEBSD))
            && elf::elf_st_type(sym_info).eq(&elf::STT_IFUNC);
        symbol.set_st_shndx(if is_ifunc {
            elf::SHN_UNDEF
        } else {
            elf::SHN_LIVEPATCH
        });
        symbol.set_st_info(sym_info);
        symbol.set_st_other(debuginfo_sym.get_st_other());
        symbol.set_st_value(debuginfo_sym.get_st_value());
        symbol.set_st_size(debuginfo_sym.get_st_size());
        trace!(
            "Found unresolved symbol {} at 0x{:x}",
            debuginfo_sym.get_st_name().to_string_lossy(),
            symbol.get_st_value()
        );
        break;
    }

    Ok(())
}

/*
 * In order to avoid external access to internal symbols, the dynamic library changes some GLOBAL symbols to the LOCAL symbols,
 * then we can't match these symbols, we change these symbols to GLOBAL here.
 */
pub fn resolve_dynamic<P: AsRef<Path>>(debuginfo: P) -> Result<()> {
    let mut debuginfo_elf = elf::write::Elf::parse(debuginfo)?;
    let debuginfo_header = debuginfo_elf.header()?;

    if debuginfo_header.get_e_type().ne(&elf::ET_DYN) {
        return Ok(());
    }

    let mut debuginfo_symbols = debuginfo_elf.symbols()?;

    for mut symbol in &mut debuginfo_symbols {
        if elf::elf_st_type(symbol.get_st_info()).ne(&elf::STT_FILE) {
            continue;
        }

        let symbol_name = symbol.get_st_name();
        if symbol_name.is_empty() {
            _resolve_dynamic(&mut debuginfo_symbols)?;
            break;
        }
    }
    Ok(())
}

fn _resolve_dynamic(debuginfo_symbols: &mut elf::write::SymbolHeaderTable) -> Result<()> {
    for mut symbol in debuginfo_symbols {
        if elf::elf_st_type(symbol.get_st_info()).eq(&elf::STT_FILE) {
            break;
        }

        let st_info = symbol.get_st_info();
        if elf::elf_st_bind(st_info).ne(&elf::STB_GLOBAL) {
            let symbol_name = symbol.get_st_name();
            trace!(
                "resolve_dynamic: set {} bind {} to 1",
                symbol_name.to_string_lossy(),
                elf::elf_st_bind(st_info)
            );

            let new_st_info = elf::elf_st_type(st_info) | (elf::STB_GLOBAL << 4);
            symbol.set_st_info(new_st_info);
        }
    }
    Ok(())
}
