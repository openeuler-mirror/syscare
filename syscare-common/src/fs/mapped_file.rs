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

use std::{
    io::{BufRead, Cursor, Read},
    os::unix::io::AsRawFd,
    path::Path,
};

use anyhow::{ensure, Context, Result};
use memmap2::{Mmap, MmapOptions};

use super::flock::{FileLock, FileLockType};

#[derive(Debug)]
pub struct MappedFile {
    _flock: FileLock,
    cursor: Cursor<Mmap>,
}

impl MappedFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file_path = path.as_ref();
        ensure!(
            !file_path.starts_with("/proc"),
            "Mmap does not support procfs"
        );

        let flock = FileLock::new(file_path, FileLockType::Shared)?;
        /*
         * SAFETY:
         * All file-backed memory map constructors are marked unsafe because of the
         * potential for Undefined Behavior (UB) using the map if the underlying file
         * is subsequently modified, in or out of process.
         * Our implementation uses shared file lock to avoid cross-process file access.
         * This mapped area would be safe.
         */
        let mmap = unsafe {
            MmapOptions::new()
                .map(flock.as_raw_fd())
                .with_context(|| format!("Failed to mmap file {}", path.as_ref().display()))?
        };

        let cursor = Cursor::new(mmap);
        Ok(Self {
            _flock: flock,
            cursor,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.cursor.get_ref()
    }
}

impl Read for MappedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl BufRead for MappedFile {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.cursor.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.cursor.consume(amt)
    }
}

#[test]
fn test() {
    use std::{fs::File, io::Read};

    const PROC_FS_PATH: &str = "/proc/version";
    const FILE_PATH: &str = "/etc/os-release";

    println!("Testing MappedFile...");
    let mut file_buf = vec![];
    let mut mapped_buf = vec![];

    let mut normal_file = File::open(FILE_PATH).expect("Failed to open normal file");
    normal_file
        .read_to_end(&mut file_buf)
        .expect("Failed to read normal file");

    let mut mapped_file = MappedFile::open(FILE_PATH).expect("Failed to open mapped file");
    mapped_file
        .read_to_end(&mut mapped_buf)
        .expect("Failed to read mapped file");

    let _map_procfs_err =
        MappedFile::open(PROC_FS_PATH).expect_err("Procfs should not be supported");

    assert_eq!(file_buf, mapped_buf);
    assert_eq!(mapped_buf, mapped_file.as_bytes());
}
