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

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Context, Result};
use memmap2::MmapOptions;
use object::{Object, ObjectKind};

use syscare_common::fs;

use super::{Endian, Endianness};

pub fn parse_file_kind<P: AsRef<Path>>(file_path: P) -> Result<ObjectKind> {
    Ok(object::File::parse(fs::MappedFile::open(&file_path)?.as_bytes())?.kind())
}

pub fn find_elf_files<P, F>(directory: P, predicate: F) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
    F: Fn(&Path, ObjectKind) -> bool,
{
    let mut elf_files = Vec::new();
    for file_path in fs::list_files(&directory, fs::TraverseOptions { recursive: true })? {
        if let Ok(obj_kind) = self::parse_file_kind(&file_path) {
            if predicate(&file_path, obj_kind) {
                elf_files.push(file_path);
            }
        }
    }
    elf_files.sort();

    Ok(elf_files)
}

pub fn check_elf(file: &File) -> Result<bool> {
    if file.metadata()?.len() < 64 {
        return Ok(false);
    }

    let mmap = unsafe { MmapOptions::new().offset(0).len(4).map(file)? };
    Ok(mmap[0..4].eq(&[0x7f, 0x45, 0x4c, 0x46]))
}

pub fn check_header(file: &File) -> Result<Endian> {
    const ELFCLASS64: u8 = 2;

    let mmap = unsafe { MmapOptions::new().offset(4).len(2).map(file)? };

    // Now we only support 64 bit
    let class = mmap.get(0..1).context("Failed to get elf class")?;
    ensure!(class[0] == ELFCLASS64, "Elf format is not class64");

    let endian = match mmap.get(1..2) {
        Some([1]) => Endian::new(Endianness::Little),
        Some([2]) => Endian::new(Endianness::Big),
        _ => bail!("Elf endian is invalid"),
    };

    Ok(endian)
}
