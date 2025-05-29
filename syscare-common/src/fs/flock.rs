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
    os::{fd::RawFd, unix::io::AsRawFd},
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
    fn new(file_path: &Path, kind: FileLockType) -> Result<Self> {
        let file = File::open(file_path)?;
        Self::acquire_flock(file.as_raw_fd(), kind)?;

        Ok(Self { file })
    }

    #[inline]
    fn acquire_flock(fd: RawFd, kind: FileLockType) -> Result<()> {
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
    fn release_flock(fd: RawFd) {
        fcntl::flock(fd, fcntl::FlockArg::Unlock).expect("Failed to release file lock");
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
        Self::release_flock(self.file.as_raw_fd());
    }
}

pub fn flock_exists<P: AsRef<Path>>(file_path: P, kind: FileLockType) -> Result<FileLock> {
    FileLock::new(file_path.as_ref(), kind)
}

pub fn flock<P: AsRef<Path>>(file_path: P, kind: FileLockType) -> Result<FileLock> {
    let file_path = file_path.as_ref();
    if !file_path.exists() {
        File::create(file_path)?;
    }
    self::flock_exists(file_path, kind)
}

#[test]
fn test() -> anyhow::Result<()> {
    use anyhow::{anyhow, ensure};
    use std::fs;

    let file_path = std::env::temp_dir().join("flock_test");
    let non_exist_file = std::env::temp_dir().join("flock_test_non_exist");

    fs::remove_file(&file_path).ok();
    fs::remove_file(&non_exist_file).ok();

    fs::write(&file_path, "flock_test")?;

    println!("Testing fs::flock_exists()...");
    println!("- Shared flock '{}'...", file_path.display());
    let shared_lock =
        self::flock_exists(&file_path, FileLockType::SharedNonBlock).map_err(|e| {
            anyhow!(
                "Failed to create shared flock '{}', {}",
                file_path.display(),
                e
            )
        })?;
    let shared_lock1 =
        self::flock_exists(&file_path, FileLockType::SharedNonBlock).map_err(|e| {
            anyhow!(
                "Failed to create shared flock '{}', {}",
                file_path.display(),
                e
            )
        })?;
    ensure!(
        self::flock_exists(&file_path, FileLockType::ExclusiveNonBlock).is_err(),
        "Exclusive flock '{}' should be failed",
        file_path.display()
    );
    drop(shared_lock);
    drop(shared_lock1);

    println!("- Exclusive flock '{}'...", file_path.display());
    let exclusive_lock =
        self::flock_exists(&file_path, FileLockType::ExclusiveNonBlock).map_err(|e| {
            anyhow!(
                "Failed to create exclusive flock '{}', {}",
                file_path.display(),
                e
            )
        })?;
    ensure!(
        self::flock_exists(&file_path, FileLockType::SharedNonBlock).is_err(),
        "Shared flock '{}' should be failed",
        file_path.display()
    );
    ensure!(
        self::flock_exists(&file_path, FileLockType::ExclusiveNonBlock).is_err(),
        "Exclusive flock '{}' should be failed",
        file_path.display()
    );
    drop(exclusive_lock);

    println!("- Non-exist flock '{}'...", non_exist_file.display());
    ensure!(
        self::flock_exists(&non_exist_file, FileLockType::SharedNonBlock).is_err(),
        "Shared flock '{}' should be failed",
        non_exist_file.display()
    );
    ensure!(
        self::flock_exists(&non_exist_file, FileLockType::ExclusiveNonBlock).is_err(),
        "Exclusive flock '{}' should be failed",
        non_exist_file.display()
    );

    println!("Testing fs::flock()...");
    println!("- Non-exist flock '{}'...", non_exist_file.display());
    let _ = self::flock(&non_exist_file, FileLockType::SharedNonBlock).map_err(|e| {
        anyhow!(
            "Failed to create shared flock '{}', {}",
            file_path.display(),
            e
        )
    })?;
    let _ = self::flock(&non_exist_file, FileLockType::ExclusiveNonBlock).map_err(|e| {
        anyhow!(
            "Failed to create exclusive flock '{}', {}",
            file_path.display(),
            e
        )
    })?;

    Ok(())
}
