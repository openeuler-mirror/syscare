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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::util::fs;

#[inline(always)]
fn find_disk<P: AsRef<Path>, S: AsRef<OsStr>>(directory: P, name: S) -> std::io::Result<PathBuf> {
    #[inline(always)]
    fn __find_disk(directory: &Path, name: &OsStr) -> std::io::Result<PathBuf> {
        let dev = fs::find_symlink(
            directory,
            name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;
        fs::canonicalize(dev)
    }

    __find_disk(directory.as_ref(), name.as_ref()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Cannot find block device by label \"{}\"",
                name.as_ref().to_string_lossy()
            ),
        )
    })
}

pub fn find_by_id<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-id", name)
}

pub fn find_by_label<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-label", name)
}

pub fn find_by_uuid<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-uuid", name)
}

pub fn find_by_partuuid<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-partuuid", name)
}

pub fn find_by_path<S: AsRef<OsStr>>(name: S) -> std::io::Result<PathBuf> {
    find_disk("/dev/disk/by-path", name)
}
