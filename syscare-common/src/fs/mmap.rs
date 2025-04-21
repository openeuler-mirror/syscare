// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{io::Result, ops::Deref, os::fd::AsRawFd, path::Path};

use memmap2::{Advice, Mmap, MmapOptions};

use super::flock::*;

#[derive(Debug)]
pub struct FileMmap {
    _lock: FileLock,
    mmap: Mmap,
}

impl FileMmap {
    fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        /*
         * SAFETY:
         * All file-backed memory map constructors are marked unsafe because of the
         * potential for Undefined Behavior (UB) using the map if the underlying file
         * is subsequently modified, in or out of process.
         * Our implementation uses shared file lock to avoid cross-process file access.
         * This mapped area would be safe.
         */
        let lock = flock(file_path, FileLockType::Shared)?;
        let mmap = unsafe { MmapOptions::new().map(lock.as_raw_fd())? };
        mmap.advise(Advice::Random)?;

        Ok(Self { _lock: lock, mmap })
    }
}

impl Deref for FileMmap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mmap
    }
}

pub fn mmap<P: AsRef<Path>>(file_path: P) -> std::io::Result<FileMmap> {
    FileMmap::new(file_path)
}

#[test]
fn test() -> anyhow::Result<()> {
    use anyhow::Context;

    const FILE_PATH: &str = "/etc/os-release";
    const SYS_FS_PATH: &str = "/sys/kernel/vmcoreinfo";
    const PROC_FS_PATH: &str = "/proc/version";

    println!("Testing FileMmap...");
    let orig_file =
        std::fs::read(FILE_PATH).with_context(|| format!("Failed to open file {}", FILE_PATH))?;
    let map_file =
        self::mmap(FILE_PATH).with_context(|| format!("Failed to mmap file {}", FILE_PATH))?;

    let _ = self::mmap(SYS_FS_PATH).expect_err("Sysfs cannot not be mapped");
    let _ = self::mmap(PROC_FS_PATH).expect_err("Procfs cannot not be mapped");

    assert_eq!(orig_file, map_file.as_ref());

    Ok(())
}
