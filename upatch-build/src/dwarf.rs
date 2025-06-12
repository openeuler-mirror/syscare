use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::Path,
    rc::Rc,
};

use anyhow::{bail, Context, Result};
use gimli::{
    constants::*, AttributeValue, DebugInfoUnitHeadersIter, DebuggingInformationEntry, DwLang,
    Dwarf, EndianRcSlice, Endianity, Reader, RunTimeEndian, SectionId, Unit, UnitOffset,
};
use object::{Endianness, Object, ObjectSection, ObjectSymbol, RelocationKind, RelocationTarget};
use once_cell::sync::Lazy;
use regex::bytes::Regex;

use syscare_common::fs::{self, FileMmap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProducerType {
    GnuAs,
    LlvmAs,
    GnuC,
    ClangC,
    GnuCxx,
    ClangCxx,
    Unknown,
}

impl std::fmt::Display for ProducerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ProducerType::GnuAs => "GNU AS",
            ProducerType::LlvmAs => "LLVM AS",
            ProducerType::GnuC => "GNU C",
            ProducerType::ClangC => "Clang C",
            ProducerType::GnuCxx => "GNU C++",
            ProducerType::ClangCxx => "Clang C++",
            ProducerType::Unknown => "Unknown",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Producer {
    pub kind: ProducerType,
    pub name: OsString,
    pub version: OsString,
}

impl Producer {
    pub fn is_assembler(&self) -> bool {
        matches!(self.kind, ProducerType::GnuAs | ProducerType::LlvmAs)
    }
}

impl std::fmt::Display for Producer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} {}",
            self.name.to_string_lossy(),
            self.version.to_string_lossy()
        ))
    }
}

pub struct ProducerParser {
    mmap: FileMmap,
    data_map: RefCell<HashMap<SectionId, Rc<[u8]>>>,
}

impl ProducerParser {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let parser = Self {
            mmap: fs::mmap(&path)
                .with_context(|| format!("Failed to mmap file {}", path.as_ref().display()))?,
            data_map: RefCell::new(HashMap::new()),
        };
        Ok(parser)
    }

    pub fn parse(&self) -> Result<ProducerIterator<impl Reader<Offset = usize> + '_>> {
        let dwarf = Dwarf::load(|section_id| -> Result<_> { self.load_section(section_id) })
            .context("Failed to load DWARF information")?;
        let headers = dwarf.units();

        Ok(ProducerIterator {
            dwarf,
            headers,
            state: None,
        })
    }
}

impl ProducerParser {
    fn load_section(&self, section_id: SectionId) -> Result<impl Reader<Offset = usize> + '_> {
        const U8_TYPE_SIZE: usize = 1;
        const U16_TYPE_SIZE: usize = 2;
        const U32_TYPE_SIZE: usize = 4;
        const U64_TYPE_SIZE: usize = 8;
        const BYTE_BIT_NUM: u8 = 8;

        let file = object::File::parse(self.mmap.as_ref())?;
        let endian = match file.endianness() {
            Endianness::Little => RunTimeEndian::Little,
            Endianness::Big => RunTimeEndian::Big,
        };

        let section_name = section_id.name();
        let section = match file.section_by_name(section_name) {
            Some(section) => section,
            None => return Ok(EndianRcSlice::new(Rc::new([]), endian)),
        };

        let mut section_data = section
            .uncompressed_data()
            .map(|slice| slice.into_owned())
            .with_context(|| format!("Failed to read section {}", section_name))?;
        for (offset, reloc) in section.relocations() {
            if let RelocationTarget::Symbol(index) = reloc.target() {
                if !matches!(reloc.kind(), RelocationKind::Absolute) {
                    continue;
                }

                let symbol = file.symbol_by_index(index)?;
                let addend = reloc.addend();
                let value = if addend >= 0 {
                    symbol.address().checked_add(addend.unsigned_abs())
                } else {
                    symbol.address().checked_sub(addend.unsigned_abs())
                }
                .context("Relocation overflow")?;

                let len = (reloc.size() / BYTE_BIT_NUM) as usize;
                let buf = &mut section_data[offset as usize..offset as usize + len];

                match len {
                    U8_TYPE_SIZE => buf[0] = value as u8,
                    U16_TYPE_SIZE => endian.write_u16(buf, value as u16),
                    U32_TYPE_SIZE => endian.write_u32(buf, value as u32),
                    U64_TYPE_SIZE => endian.write_u64(buf, value),
                    _ => bail!("Invalid relocation length"),
                }
            } else {
                bail!("Unsupported relocation type");
            }
        }

        let bytes: Rc<[u8]> = Rc::from(section_data.into_boxed_slice());
        self.data_map.borrow_mut().insert(section_id, bytes.clone());

        Ok(EndianRcSlice::new(bytes, endian))
    }

    fn parse_producer_attr<R>(
        dwarf: &Dwarf<R>,
        unit: &Unit<R>,
        attr: AttributeValue<R>,
    ) -> Result<OsString>
    where
        R: Reader<Offset = usize>,
    {
        let attr = dwarf
            .attr_string(unit, attr)
            .context("Cannot find attribute string")?;
        let slice = attr
            .to_slice()
            .context("Failed to read attribute string data")?;

        Ok(OsStr::from_bytes(&slice).to_os_string())
    }

    fn parse_producer_name(str: &OsStr) -> Option<&OsStr> {
        /*
         * Matches name in producer string
         * eg. GNU C17 12.3.1 (openEuler 12.3.1-62.oe2403sp1) -> GNU C17
         * eg. clang version 17.0.6 (17.0.6-30-oe2043sp1)     -> clang
         */
        static PRODUCER_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^((?:\s?[A-Za-z]+\d*)+)").expect("Invalid producer name regex")
        });

        PRODUCER_NAME_REGEX
            .captures(str.as_bytes())
            .and_then(|captures| captures.get(1))
            .map(|matched| matched.as_bytes())
            .map(|bytes| bytes.strip_suffix(b" version").unwrap_or(bytes))
            .map(OsStr::from_bytes)
    }

    fn parse_producer_version(str: &OsStr) -> Option<&OsStr> {
        /*
         * Matches version in producer string
         * eg. GNU C17 12.3.1 (openEuler 12.3.1-62.oe2403sp1) -> 12.3.1
         * eg. clang version 17.0.6 (17.0.6-30-oe2043sp1)     -> 17.0.6
         */
        static PRODUCER_VERSION_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\d+(?:\.\d+)+)").expect("Invalid producer version regex"));

        PRODUCER_VERSION_REGEX
            .captures(str.as_bytes())
            .and_then(|captures| captures.get(1))
            .map(|matched| matched.as_bytes())
            .map(OsStr::from_bytes)
    }

    fn parse_producer_type<R>(str: &OsStr, attr: AttributeValue<R>) -> Result<ProducerType>
    where
        R: Reader<Offset = usize>,
    {
        const DW_LANGS_AS: &[DwLang] = &[
            DW_LANG_Mips_Assembler,
            DW_LANG_SUN_Assembler,
            DW_LANG_ALTIUM_Assembler,
        ];
        const DW_LANGS_C: &[DwLang] = &[
            DW_LANG_C,
            DW_LANG_C89,
            DW_LANG_C99,
            DW_LANG_C11,
            DW_LANG_C17,
        ];
        const DW_LANGS_CXX: &[DwLang] = &[
            DW_LANG_C_plus_plus,
            DW_LANG_C_plus_plus_03,
            DW_LANG_C_plus_plus_11,
            DW_LANG_C_plus_plus_14,
            DW_LANG_C_plus_plus_17,
            DW_LANG_C_plus_plus_20,
        ];
        const GNU_PRODUCER_PREFIX: &str = "GNU";

        let lang = match attr {
            AttributeValue::Language(lang) => lang,
            _ => bail!("Unexpected attribute type"),
        };
        let is_gnu = str.to_string_lossy().starts_with(GNU_PRODUCER_PREFIX);
        let kind = match lang {
            lang if DW_LANGS_AS.contains(&lang) => {
                if is_gnu {
                    ProducerType::GnuAs
                } else {
                    ProducerType::LlvmAs
                }
            }
            lang if DW_LANGS_C.contains(&lang) => {
                if is_gnu {
                    ProducerType::GnuC
                } else {
                    ProducerType::ClangC
                }
            }
            lang if DW_LANGS_CXX.contains(&lang) => {
                if is_gnu {
                    ProducerType::GnuCxx
                } else {
                    ProducerType::ClangCxx
                }
            }
            _ => ProducerType::Unknown,
        };

        Ok(kind)
    }

    fn parse_producer<R>(
        dwarf: &Dwarf<R>,
        unit: &Unit<R>,
        die: DebuggingInformationEntry<R>,
    ) -> Result<Option<Producer>>
    where
        R: Reader<Offset = usize>,
    {
        if die.tag() != DW_TAG_compile_unit {
            return Ok(None);
        }

        let str = match die.attr_value(DW_AT_producer)? {
            Some(attr) => Self::parse_producer_attr(dwarf, unit, attr)?,
            None => bail!("Invalid DW_AT_producer attribute"),
        };
        let kind = match die.attr_value(DW_AT_language)? {
            Some(attr) => Self::parse_producer_type(&str, attr)?,
            _ => bail!("Invalid DW_AT_language attribute"),
        };
        let name = Self::parse_producer_name(&str).context("Invalid producer name")?;
        let version = Self::parse_producer_version(&str).context("Invalid producer version")?;

        Ok(Some(Producer {
            kind,
            name: name.to_os_string(),
            version: version.to_os_string(),
        }))
    }
}

pub struct ProducerIterator<R: Reader> {
    dwarf: Dwarf<R>,
    headers: DebugInfoUnitHeadersIter<R>,
    state: Option<(Unit<R>, Vec<UnitOffset>)>,
}

impl<R: Reader<Offset = usize>> ProducerIterator<R> {
    fn current(&self) -> Result<Option<(&Unit<R>, DebuggingInformationEntry<R>)>> {
        if let Some((unit, offsets)) = &self.state {
            if let Some(offset) = offsets.last() {
                return Ok(Some((unit, unit.entry(*offset)?)));
            }
        }
        Ok(None)
    }

    fn has_die(&self) -> bool {
        self.state
            .as_ref()
            .map(|(_, offsets)| !offsets.is_empty())
            .unwrap_or(false)
    }

    fn next_die(&mut self) {
        if let Some((_, offsets)) = &mut self.state {
            offsets.pop();
        }
    }

    fn next_unit(&mut self) -> Result<()> {
        if let Some(header) = self.headers.next()? {
            let unit = self.dwarf.unit(header)?;

            let mut offsets = Vec::new();
            let mut cursor = unit.entries();
            while let Some((_, entry)) = cursor.next_dfs()? {
                offsets.push(entry.offset());
            }
            offsets.reverse();

            self.state = Some((unit, offsets));
        }

        Ok(())
    }
}

impl<R: Reader<Offset = usize>> Iterator for ProducerIterator<R> {
    type Item = Result<Producer>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let producer = match self.current() {
                Ok(Some((unit, die))) => {
                    ProducerParser::parse_producer(&self.dwarf, unit, die).transpose()
                }
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            };

            self.next_die();

            if producer.is_some() {
                return producer;
            }

            if !self.has_die() {
                if let Err(e) = self.next_unit() {
                    return Some(Err(e));
                }
                if !self.has_die() {
                    return None;
                }
            }
        }
    }
}
