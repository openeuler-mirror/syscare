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

use std::fs::OpenOptions;
use std::path::Path;

use anyhow::bail;
use anyhow::Result;
use memmap2::{Mmap, MmapOptions};

use super::{
    super::{check_elf, check_header, Endian, HeaderRead, SectionRead, SymbolHeader64, SHT_SYMTAB},
    Header, SectionHeaderTable, SymbolHeaderTable,
};

#[derive(Debug)]
pub struct Elf {
    mmap: Mmap,
    endian: Endian,
}

impl Elf {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new().read(true).open(&path)?;
        check_elf(&file)?;

        let endian = check_header(&file)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self { mmap, endian })
    }

    pub fn header(&self) -> Result<Header<'_>> {
        Ok(Header::from(&self.mmap, self.endian))
    }

    pub fn sections(&self) -> Result<SectionHeaderTable<'_>> {
        let header = self.header()?;
        let offset = header.get_e_shoff() as usize;
        let num = header.get_e_shnum() as usize;
        let shentsize = header.get_e_shentsize() as usize;
        Ok(SectionHeaderTable::from(
            &self.mmap,
            self.endian,
            offset,
            shentsize,
            num,
        ))
    }

    pub fn symbols(&self) -> Result<SymbolHeaderTable<'_>> {
        let sections = self.sections()?;
        for section in sections.clone() {
            if section.get_sh_type().eq(&SHT_SYMTAB) {
                let offset = section.get_sh_offset() as usize;
                let size_sum = section.get_sh_size() as usize;
                let size = std::mem::size_of::<SymbolHeader64>();
                let num = size_sum / size;
                let strtab_offset = sections
                    .get(section.get_sh_link() as usize)?
                    .get_sh_offset() as usize;

                return Ok(SymbolHeaderTable::from(
                    &self.mmap,
                    self.endian,
                    strtab_offset,
                    offset,
                    size,
                    num,
                ));
            }
        }
        bail!("Cannot find symtab");
    }
}
