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

use anyhow::Result;
use gimli::{
    constants, Attribute, AttributeValue, EndianSlice, Endianity, Reader, RunTimeEndian, SectionId,
};
use indexmap::{IndexMap, IndexSet};
use log::trace;
use object::{
    File, Object, ObjectSection, ObjectSymbol, Relocation, RelocationKind, RelocationTarget,
    Section,
};
use typed_arena::Arena;

use syscare_common::{ffi::OsStrExt, fs::MappedFile};

use relocate::Relocate;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CompileUnit {
    producer: OsString,   // DW_AT_producer
    compile_dir: PathBuf, // DW_AT_comp_dir
    file_name: PathBuf,   // DW_AT_name
}

impl CompileUnit {
    pub fn producer(&self) -> OsString {
        self.producer
            .split('-')
            .next()
            .map(|s| s.trim().to_os_string())
            .unwrap_or_else(|| self.producer.clone())
    }

    pub fn producer_type(&self) -> ProducerType {
        ProducerType::from(&self.producer)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerType {
    C,
    Cxx,
    As,
    Unknown,
}

impl<S: AsRef<OsStr>> From<S> for ProducerType {
    fn from(s: S) -> Self {
        if s.as_ref().contains("C++") {
            ProducerType::Cxx
        } else if s.as_ref().contains("C") {
            ProducerType::C
        } else if s.as_ref().contains("AS") {
            ProducerType::As
        } else {
            ProducerType::Unknown
        }
    }
}

impl std::fmt::Display for ProducerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProducerType::C => "C",
            ProducerType::Cxx => "CXX",
            ProducerType::As => "AS",
            ProducerType::Unknown => "Unknown",
        })
    }
}

pub struct Dwarf {
    pub units: IndexSet<CompileUnit>,
}

impl Dwarf {
    pub fn parse<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let mmap = MappedFile::open(file_path)?;
        let object = File::parse(mmap.as_bytes())?;
        let endian = if object.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        Ok(Self {
            units: Self::parse_compile_units(&object, endian)?,
        })
    }

    pub fn producers(&self) -> IndexSet<OsString> {
        self.units.iter().map(|unit| unit.producer()).collect()
    }

    pub fn producer_types(&self) -> IndexSet<ProducerType> {
        self.units.iter().map(|unit| unit.producer_type()).collect()
    }
}

impl Dwarf {
    fn add_relocations(
        relocations: &mut IndexMap<usize, Relocation>,
        file: &File,
        section: &Section,
    ) {
        const INVALID_SECTION_NAME: &str = ".invalid";

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
                                trace!("Relocation with invalid symbol for section {} at offset 0x{:08x}",
                                    section.name().unwrap_or(INVALID_SECTION_NAME), offset
                                );
                            }
                        }
                    }
                    if relocations.insert(offset, relocation).is_some() {
                        trace!(
                            "Multiple relocations for section {} at offset 0x{:08x}",
                            section.name().unwrap_or(INVALID_SECTION_NAME),
                            offset
                        );
                    }
                }
                _ => {
                    trace!(
                        "Unsupported relocation for section {} at offset 0x{:08x}",
                        section.name().unwrap_or(INVALID_SECTION_NAME),
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
        let mut relocation_map = IndexMap::new();
        let name = Some(id.name());
        let data = match name.and_then(|section_name| file.section_by_name(section_name)) {
            Some(ref section) => {
                // DWO sections never have relocations, so don't bother.
                Self::add_relocations(&mut relocation_map, file, section);
                section.uncompressed_data()?
            }
            // Use a non-zero capacity so that `ReaderOffsetId`s are unique.
            None => Cow::Owned(Vec::with_capacity(1)),
        };
        let data_ref = (*arena_data.alloc(data)).borrow();
        let reader = EndianSlice::new(data_ref, endian);
        let section = reader;
        let relocations = (*arena_relocations.alloc(relocation_map)).borrow();
        Ok(Relocate {
            relocations,
            section,
            reader,
        })
    }

    fn parse_compile_units(file: &File, endian: RunTimeEndian) -> Result<IndexSet<CompileUnit>> {
        let arena_data = Arena::new();
        let arena_relocations = Arena::new();

        // Load a section and return as `Cow<[u8]>`.
        let mut load_section = |id: SectionId| -> Result<_> {
            Self::load_file_section(id, file, endian, &arena_data, &arena_relocations)
        };

        let dwarf = gimli::Dwarf::load(&mut load_section)?;

        Self::build_compile_units(&dwarf)
    }

    fn build_compile_units<R: Reader>(dwarf: &gimli::Dwarf<R>) -> Result<IndexSet<CompileUnit>> {
        let mut result = IndexSet::new();
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
                let mut element = CompileUnit {
                    producer: OsString::new(),
                    compile_dir: PathBuf::new(),
                    file_name: PathBuf::new(),
                };

                while let Some(attr) = attrs.next()? {
                    match attr.name() {
                        constants::DW_AT_comp_dir => {
                            element.compile_dir.push(&Self::attr_value(&attr, dwarf));
                        }
                        constants::DW_AT_name => {
                            element.file_name.push(&Self::attr_value(&attr, dwarf));
                        }
                        constants::DW_AT_producer => {
                            element.producer.push(&Self::attr_value(&attr, dwarf));
                        }
                        _ => continue,
                    }
                }

                result.insert(element);
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
