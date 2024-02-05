use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use anyhow::Result;
use object::{elf, read::elf::FileHeader, NativeFile, Object, ObjectSymbol};

pub struct ElfResolver<'a> {
    elf: NativeFile<'a, &'a [u8]>,
}

impl<'a> ElfResolver<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Ok(Self {
            elf: NativeFile::parse(data)?,
        })
    }
}

impl ElfResolver<'_> {
    pub fn dependencies(&self) -> Result<Vec<&OsStr>> {
        let endian = self.elf.endian();
        let data = self.elf.data();

        let header = self.elf.raw_header();
        let sections = header.sections(endian, data)?;

        if let Some((section, index)) = sections.dynamic(endian, data)? {
            let strtab = sections.strings(endian, data, index)?;
            let libs = section
                .iter()
                .filter_map(|entry| {
                    let tag = entry.d_tag.get(endian) as u32;
                    if tag != elf::DT_NEEDED {
                        return None;
                    }

                    let value = entry.d_val.get(endian) as u32;
                    if let Ok(lib_name) = strtab.get(value).map(OsStr::from_bytes) {
                        return Some(lib_name);
                    }
                    None
                })
                .collect::<Vec<_>>();

            return Ok(libs);
        }

        Ok(vec![])
    }

    pub fn find_symbol_addr(&self, symbol_name: &str) -> Result<Option<u64>> {
        let symbols = self.elf.dynamic_symbols();
        for sym in symbols {
            if let Ok(sym_name) = sym.name() {
                if sym_name == symbol_name {
                    return Ok(Some(sym.address()));
                }
            }
        }

        Ok(None)
    }
}
