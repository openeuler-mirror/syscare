use std::path::Path;

use crate::log::*;

use crate::elf::*;

pub fn resolve<P: AsRef<Path>, Q: AsRef<Path>>(debug_info: P, patch: Q) -> std::io::Result<()> {
    let mut patch_elf = write::Elf::parse(patch)?;
    let mut debug_info_elf = read::Elf::parse(debug_info)?;

    let debug_info_symbols = &mut debug_info_elf.symbols()?;

    for mut symbol in &mut patch_elf.symbols()? {
        /* No need to handle section symbol */
        let sym_info = symbol.get_st_info();
        if elf_st_type(sym_info) == STT_SECTION {
            continue;
        }

        let sym_other = symbol.get_st_other();
        if sym_other & SYM_OTHER != 0 {
            // TODO: we can delete these symbol's section here.
            symbol.set_st_other(sym_other & !SYM_OTHER);
            match symbol.get_st_value() {
                0 => symbol.set_st_shndx(SHN_UNDEF),
                _=> symbol.set_st_shndx(SHN_LIVEPATCH),
            };
        }
        else if symbol.get_st_shndx() == SHN_UNDEF {
            if elf_st_bind(sym_info) == STB_LOCAL {
                /* only partly resolved undefined symbol could have st_value */
                if symbol.get_st_value() != 0 {
                    symbol.set_st_shndx(SHN_LIVEPATCH);
                }
            }
            else {
                __partly_resolve_patch(&mut symbol, debug_info_symbols)?;
            }
        }
    }
    Ok(())
}

fn __partly_resolve_patch(symbol: &mut write::SymbolHeader, debug_info_symbols: &mut read::SymbolHeaderTable) -> std::io::Result<()> {
    debug_info_symbols.reset(0);
    for mut debug_info_symbol in debug_info_symbols {
        /* No need to handle section symbol */
        let sym_info = debug_info_symbol.get_st_info();
        if elf_st_type(sym_info) == STT_SECTION {
            continue;
        }

        let debug_info_name = debug_info_symbol.get_st_name();

        if debug_info_name.ne(symbol.get_st_name()) {
            continue;
        }

        /* leave it to be handled in running time */
        if debug_info_symbol.get_st_shndx() == SHN_UNDEF {
            continue;
        }

        symbol.set_st_shndx(SHN_LIVEPATCH);
        symbol.set_st_info(sym_info);
        symbol.set_st_other(debug_info_symbol.get_st_other());
        symbol.set_st_value(debug_info_symbol.get_st_value());
        symbol.set_st_size(debug_info_symbol.get_st_size());
        trace!("found unresolved symbol {:?} at 0x{:x}", debug_info_symbol.get_st_name(), symbol.get_st_value());
        break;
    }

    Ok(())
}