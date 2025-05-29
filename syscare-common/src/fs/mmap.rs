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

use std::{ops::Deref, os::unix::io::AsRawFd, path::Path};

use memmap2::{Advice, Mmap, MmapOptions};

use super::flock;

#[derive(Debug)]
pub struct FileMmap {
    _lock: flock::FileLock,
    mmap: Mmap,
}

impl Deref for FileMmap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mmap
    }
}

pub fn mmap<P: AsRef<Path>>(file_path: P) -> std::io::Result<FileMmap> {
    /*
     * SAFETY:
     * All file-backed memory map constructors are marked unsafe because of the
     * potential for Undefined Behavior (UB) using the map if the underlying file
     * is subsequently modified, in or out of process.
     * Our implementation uses shared file lock to avoid cross-process file access.
     * This mapped area would be safe.
     */
    let lock = flock::flock_exists(file_path, flock::FileLockType::Shared)?;
    let mmap = unsafe { MmapOptions::new().map(lock.as_raw_fd())? };
    mmap.advise(Advice::Random)?;

    Ok(FileMmap { _lock: lock, mmap })
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::*;

    #[test]
    fn mmap_file() -> std::io::Result<()> {
        let file_path = std::env::temp_dir().join("mmap_test");
        fs::remove_file(&file_path).ok();
        fs::write(&file_path, "mmap_test")?;

        let orig_file = fs::read(&file_path)?;
        let map_file = self::mmap(&file_path)?;
        assert_eq!(orig_file, map_file.as_ref());

        fs::remove_file(&file_path)?;
        Ok(())
    }

    #[test]
    fn mmap_non_exists_file() {
        const NON_EXISTS_FILE: &str = "/non_exists_file";
        fs::remove_file(NON_EXISTS_FILE).ok();

        assert!(self::mmap(NON_EXISTS_FILE).is_err(),);
    }

    #[test]
    fn mmap_proc_file() {
        const PROC_FS_PATH: &str = "/proc/version";
        assert!(self::mmap(PROC_FS_PATH).is_err(),);
    }

    #[test]
    fn mmap_sys_file() {
        const SYS_FS_PATH: &str = "/sys/kernel/vmcoreinfo";
        assert!(self::mmap(SYS_FS_PATH).is_err(),);
    }
}
