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

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use nix::errno::Errno;
use nix::unistd;

pub fn id() -> i32 {
    unistd::getpid().as_raw()
}

pub fn path() -> std::io::Result<PathBuf> {
    std::env::current_exe()
}

pub fn name() -> std::io::Result<OsString> {
    self::path()?
        .file_name()
        .map(OsStr::to_os_string)
        .ok_or(std::io::Error::from(Errno::EINVAL))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_id() {
        let pid = self::id();
        println!("pid: {}", pid);
        assert!(pid > 1)
    }

    #[test]
    fn test_path() {
        let path = self::path().expect("Failed to get executable path");
        println!("path: {}", path.display());
        assert!(path.exists());
    }

    #[test]
    fn test_name() {
        let name = self::name().expect("Failed to get executable name");
        println!("name: {}", name.to_string_lossy());
        assert!(!name.is_empty());
    }
}
