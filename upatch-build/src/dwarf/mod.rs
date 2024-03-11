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

// mod dump;
mod relocate;

use std::{
    borrow::{Borrow, Cow},
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt as UnixOsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use gimli::{
    constants, Attribute, AttributeValue, EndianSlice, Endianity, Reader, RunTimeEndian, SectionId,
};
use indexmap::{IndexMap, IndexSet};
use log::{debug, trace};
use object::{
    File, Object, ObjectSection, ObjectSymbol, Relocation, RelocationKind, RelocationTarget,
    Section,
};
use typed_arena::Arena;

use syscare_common::ffi::OsStrExt;

use relocate::Relocate;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct DwarfCompileUnit {
    pub DW_AT_producer: OsString,
    pub DW_AT_comp_dir: PathBuf,
    pub DW_AT_name: PathBuf,
}

pub struct Dwarf;

impl Dwarf {
    pub fn parse<P: AsRef<Path>>(elf: P) -> Result<Vec<DwarfCompileUnit>> {
        // use mmap here, but depend on some devices
        let elf = elf.as_ref();
        let file = std::fs::File::open(elf)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let object = File::parse(&*mmap)?;
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        Self::get_files(&object, endian)
    }

    pub fn parse_source_file<P: AsRef<Path>>(object: P) -> Result<PathBuf> {
        let source_files = Dwarf::parse(&object)
            .with_context(|| format!("Failed to read dwarf of {}", object.as_ref().display()))?
            .into_iter()
            .filter_map(|dwarf| {
                let file_path = dwarf.DW_AT_comp_dir.join(&dwarf.DW_AT_name);
                match file_path.exists() {
                    true => Some(file_path),
                    false => None,
                }
            })
            .collect::<IndexSet<_>>();

        match source_files.len() {
            1 => Ok(source_files[0].clone()),
            0 => bail!("Object does not contain source file"),
            _ => bail!("Object contains to too many source files"),
        }
    }

    pub fn parse_compiler_name<P: AsRef<Path>>(object: P) -> Result<OsString> {
        let compiler_names = Dwarf::parse(&object)
            .with_context(|| format!("Failed to read dwarf of {}", object.as_ref().display()))?
            .into_iter()
            .filter_map(|dwarf| {
                dwarf
                    .DW_AT_producer
                    .split('-')
                    .next()
                    .map(|s| s.trim().to_os_string())
            })
            .collect::<IndexSet<_>>();

        match compiler_names.len() {
            1 => Ok(compiler_names[0].clone()),
            0 => bail!("Object does not contain compiler name"),
            _ => bail!("Object contains to too many compiler names"),
        }
    }
}

impl Dwarf {
    fn add_relocations(
        relocations: &mut IndexMap<usize, Relocation>,
        file: &File,
        section: &Section,
    ) {
        for (offset64, mut relocation) in section.relocations() {
            let offset = offset64 as usize;
            if offset as u64 != offset64 {
                continue;
            }
            match relocation.kind() {
                RelocationKind::Absolute => {
                    if let RelocationTarget::Symbol(symbol_idx) = relocation.target() {
                        match file.symbol_by_index(symbol_idx) {
                            Ok(symbol) => {
                                let addend =
                                    symbol.address().wrapping_add(relocation.addend() as u64);
                                relocation.set_addend(addend as i64);
                            }
                            Err(_) => {
                                trace!( "Relocation with invalid symbol for section {} at offset 0x{:08x}",
                                    section.name().unwrap(),
                                    offset
                                );
                            }
                        }
                    }
                    if relocations.insert(offset, relocation).is_some() {
                        trace!(
                            "Multiple relocations for section {} at offset 0x{:08x}",
                            section.name().unwrap(),
                            offset
                        );
                    }
                }
                _ => {
                    trace!(
                        "Unsupported relocation for section {} at offset 0x{:08x}",
                        section.name().unwrap(),
                        offset
                    );
                }
            }
        }
    }

    fn load_file_section<'input, 'arena, Endian: Endianity>(
        id: SectionId,
        file: &File<'input>,
        endian: Endian,
        arena_data: &'arena Arena<Cow<'input, [u8]>>,
        arena_relocations: &'arena Arena<IndexMap<usize, Relocation>>,
    ) -> Result<Relocate<'arena, EndianSlice<'arena, Endian>>> {
        let mut relocations = IndexMap::new();
        let name = Some(id.name());
        let data = match name.and_then(|name| file.section_by_name(name)) {
            Some(ref section) => {
                // DWO sections never have relocations, so don't bother.
                Self::add_relocations(&mut relocations, file, section);
                section.uncompressed_data()?
            }
            // Use a non-zero capacity so that `ReaderOffsetId`s are unique.
            None => Cow::Owned(Vec::with_capacity(1)),
        };
        let data_ref = (*arena_data.alloc(data)).borrow();
        let reader = EndianSlice::new(data_ref, endian);
        let section = reader;
        let relocations = (*arena_relocations.alloc(relocations)).borrow();
        Ok(Relocate {
            relocations,
            section,
            reader,
        })
    }

    fn get_files(file: &File, endian: RunTimeEndian) -> Result<Vec<DwarfCompileUnit>> {
        let arena_data = Arena::new();
        let arena_relocations = Arena::new();

        // Load a section and return as `Cow<[u8]>`.
        let mut load_section = |id: SectionId| -> Result<_> {
            Self::load_file_section(id, file, endian, &arena_data, &arena_relocations)
        };

        let dwarf = gimli::Dwarf::load(&mut load_section)?;

        Self::__get_files(&dwarf)
    }

    fn __get_files<R: Reader>(dwarf: &gimli::Dwarf<R>) -> Result<Vec<DwarfCompileUnit>> {
        let mut result = Vec::new();
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;
            let mut entries = unit.entries();
            while let Some((_, entry)) = entries.next_dfs()? {
                if entry.tag() != constants::DW_TAG_compile_unit {
                    break;
                }
                // Iterate over the attributes in the DIE.
                let mut attrs = entry.attrs();
                let mut element = DwarfCompileUnit {
                    DW_AT_producer: OsString::new(),
                    DW_AT_comp_dir: PathBuf::new(),
                    DW_AT_name: PathBuf::new(),
                };

                while let Some(attr) = attrs.next()? {
                    match attr.name() {
                        constants::DW_AT_comp_dir => {
                            element.DW_AT_comp_dir.push(&Self::attr_value(&attr, dwarf));
                        }
                        constants::DW_AT_name => {
                            element.DW_AT_name.push(&Self::attr_value(&attr, dwarf));
                        }
                        constants::DW_AT_producer => {
                            element.DW_AT_producer.push(&Self::attr_value(&attr, dwarf));
                        }
                        _ => continue,
                    }
                }
                match element.DW_AT_producer.contains("AS")
                    || element.DW_AT_name.extension().eq(&Some(OsStr::new(".s")))
                    || element.DW_AT_name.extension().eq(&Some(OsStr::new(".S")))
                {
                    true => debug!(
                        "Warning: Skipped assemble file {}",
                        element.DW_AT_comp_dir.join(element.DW_AT_name).display()
                    ),
                    false => result.push(element),
                };
            }
        }
        Ok(result)
    }

    fn attr_value<R: Reader>(attr: &Attribute<R>, dwarf: &gimli::Dwarf<R>) -> OsString {
        let value = attr.value();
        match value {
            AttributeValue::DebugLineStrRef(offset) => {
                if let Ok(s) = dwarf.debug_line_str.get_str(offset) {
                    OsStr::from_bytes(&s.to_slice().ok().unwrap_or_default()).to_os_string()
                } else {
                    OsString::default()
                }
            }
            AttributeValue::DebugStrRef(offset) => {
                if let Ok(s) = dwarf.debug_str.get_str(offset) {
                    OsStr::from_bytes(&s.to_slice().ok().unwrap_or_default()).to_os_string()
                } else {
                    OsString::default()
                }
            }
            _ => OsString::default(),
        }
    }
}
