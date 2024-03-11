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

use std::fs::File;

use anyhow::{bail, ensure, Context, Result};
use memmap2::MmapOptions;

use super::{Endian, Endianness};

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
