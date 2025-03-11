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
    fs::File,
    os::unix::io::{AsRawFd, RawFd},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use nix::fcntl;

#[derive(Debug)]
pub enum FileLockType {
    Shared,
    Exclusive,
    SharedNonBlock,
    ExclusiveNonBlock,
}

#[derive(Debug)]
pub struct FileLock {
    path: PathBuf,
    file: File,
    kind: FileLockType,
}

impl FileLock {
    pub fn new<P: AsRef<Path>>(path: P, kind: FileLockType) -> Result<Self> {
        let file_path = path.as_ref();

        let file = if file_path.is_file() {
            File::open(file_path)
                .map_err(|e| anyhow!("Failed to open lock file {}, {}", file_path.display(), e))?
        } else {
            File::create(file_path)
                .map_err(|e| anyhow!("Failed to create lock file {}, {}", file_path.display(), e))?
        };

        let flock = Self {
            path: file_path.to_path_buf(),
            file,
            kind,
        };
        flock.acquire_lock()?;

        Ok(flock)
    }
}

impl FileLock {
    #[inline]
    fn acquire_lock(&self) -> Result<()> {
        let fd = self.file.as_raw_fd();
        let arg = match self.kind {
            FileLockType::Shared => fcntl::FlockArg::LockShared,
            FileLockType::Exclusive => fcntl::FlockArg::LockExclusive,
            FileLockType::SharedNonBlock => fcntl::FlockArg::LockSharedNonblock,
            FileLockType::ExclusiveNonBlock => fcntl::FlockArg::LockExclusiveNonblock,
        };
        fcntl::flock(fd, arg)
            .with_context(|| format!("Failed to acquire flock on {}", self.path.display()))
    }

    #[inline]
    fn release_lock(&self) -> Result<()> {
        let fd = self.file.as_raw_fd();
        let arg = fcntl::FlockArg::Unlock;
        fcntl::flock(fd, arg)
            .with_context(|| format!("Failed to release flock on {}", self.path.display()))
    }
}

impl AsRawFd for FileLock {
    fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        self.release_lock().ok();
    }
}

#[test]
fn test() {
    use std::fs;

    const EXIST_FILE: &str = "/etc/os-release";
    const NON_EXIST_FILE: &str = "/tmp/flock_test";

    println!("Testing exclusive flock on {}...", NON_EXIST_FILE);
    fs::remove_file(NON_EXIST_FILE).ok();

    let exclusive_lock = FileLock::new(NON_EXIST_FILE, FileLockType::ExclusiveNonBlock)
        .expect("Failed to create exclusive flock");
    drop(exclusive_lock);

    println!("Testing shared flock on {}...", NON_EXIST_FILE);
    fs::remove_file(NON_EXIST_FILE).ok();
    let shared_lock = FileLock::new(NON_EXIST_FILE, FileLockType::SharedNonBlock)
        .expect("Failed to create shared flock");
    drop(shared_lock);

    fs::remove_file(NON_EXIST_FILE).ok();

    println!("Testing exclusive flock on {}...", EXIST_FILE);
    let exclusive_lock = FileLock::new(EXIST_FILE, FileLockType::ExclusiveNonBlock)
        .expect("Failed to create exclusive flock");
    let _exclusive_err = FileLock::new(EXIST_FILE, FileLockType::ExclusiveNonBlock)
        .expect_err("Exclusive flock should be failed");
    let _shared_err = FileLock::new(EXIST_FILE, FileLockType::SharedNonBlock)
        .expect_err("Shared flock should be failed");

    drop(exclusive_lock);

    println!("Testing shared flock on {}...", EXIST_FILE);
    let shared_lock1 = FileLock::new(EXIST_FILE, FileLockType::SharedNonBlock)
        .expect("Failed to create shared flock");
    let shared_lock2 = FileLock::new(EXIST_FILE, FileLockType::SharedNonBlock)
        .expect("Failed to create shared flock");
    let _exclusive_err = FileLock::new(EXIST_FILE, FileLockType::ExclusiveNonBlock)
        .expect_err("Exclusive flock should be failed");
    let shared_lock3 = FileLock::new(EXIST_FILE, FileLockType::SharedNonBlock)
        .expect("Failed to create shared flock");

    drop(shared_lock1);
    drop(shared_lock2);
    drop(shared_lock3);
}
