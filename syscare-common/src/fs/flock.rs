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
    io::Result,
    ops::{Deref, DerefMut},
    os::unix::io::AsRawFd,
    path::Path,
};

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
    file: File,
}

impl FileLock {
    fn new<P: AsRef<Path>>(file_path: P, kind: FileLockType) -> Result<Self> {
        let file_path = file_path.as_ref();
        let flock = Self {
            file: if file_path.exists() {
                File::open(file_path)?
            } else {
                File::create(file_path)?
            },
        };
        flock.acquire(kind)?;

        Ok(flock)
    }

    #[inline]
    fn acquire(&self, kind: FileLockType) -> Result<()> {
        let fd = self.file.as_raw_fd();
        let arg = match kind {
            FileLockType::Shared => fcntl::FlockArg::LockShared,
            FileLockType::Exclusive => fcntl::FlockArg::LockExclusive,
            FileLockType::SharedNonBlock => fcntl::FlockArg::LockSharedNonblock,
            FileLockType::ExclusiveNonBlock => fcntl::FlockArg::LockExclusiveNonblock,
        };
        fcntl::flock(fd, arg)?;

        Ok(())
    }

    #[inline]
    fn release(&self) {
        let fd = self.file.as_raw_fd();
        let arg = fcntl::FlockArg::Unlock;
        fcntl::flock(fd, arg).expect("Failed to release file lock");
    }
}

impl Deref for FileLock {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for FileLock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        self.release();
    }
}

pub fn flock<P: AsRef<Path>>(file_path: P, kind: FileLockType) -> Result<FileLock> {
    FileLock::new(file_path, kind)
}

#[test]
fn test() -> anyhow::Result<()> {
    use anyhow::{ensure, Context};

    use std::fs;

    let file_path = std::env::temp_dir().join("flock_test");
    fs::remove_file(&file_path)?;

    println!("Testing shared flock on {}...", file_path.display());
    let shared_lock = self::flock(&file_path, FileLockType::SharedNonBlock)
        .context("Failed to create shared flock")?;
    let shared_lock1 = self::flock(&file_path, FileLockType::SharedNonBlock)
        .context("Failed to create shared flock")?;
    ensure!(
        self::flock(&file_path, FileLockType::ExclusiveNonBlock).is_err(),
        "Exclusive flock should be failed"
    );
    drop(shared_lock);
    drop(shared_lock1);

    println!("Testing exclusive flock on {}...", file_path.display());
    let exclusive_lock = self::flock(&file_path, FileLockType::ExclusiveNonBlock)
        .context("Failed to create exclusive flock")?;
    ensure!(
        self::flock(&file_path, FileLockType::SharedNonBlock).is_err(),
        "Shared flock should be failed"
    );
    ensure!(
        self::flock(&file_path, FileLockType::ExclusiveNonBlock).is_err(),
        "Exclusive flock should be failed"
    );

    drop(exclusive_lock);

    Ok(())
}
