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

use std::ffi::OsString;

use nix::sys::utsname;

#[inline(always)]
fn sysinfo() -> utsname::UtsName {
    utsname::uname().expect("Failed to get system infomation")
}

pub fn sysname() -> OsString {
    self::sysinfo().sysname().to_os_string()
}

pub fn hostname() -> OsString {
    self::sysinfo().nodename().to_os_string()
}

pub fn release() -> OsString {
    self::sysinfo().release().to_os_string()
}

pub fn version() -> OsString {
    self::sysinfo().version().to_os_string()
}

pub fn arch() -> OsString {
    self::sysinfo().machine().to_os_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sysname() {
        let sysname = self::sysname();
        println!("sysname: {}", sysname.to_string_lossy());
        assert!(!sysname.is_empty());
    }

    #[test]
    fn test_hostname() {
        let hostname = self::hostname();
        println!("hostname: {}", hostname.to_string_lossy());
        assert!(!hostname.is_empty());
    }

    #[test]
    fn test_release() {
        let release = self::release();
        println!("release: {}", release.to_string_lossy());
        assert!(!release.is_empty());
    }

    #[test]
    fn test_version() {
        let version = self::version();
        println!("version: {}", version.to_string_lossy());
        assert!(!version.is_empty());
    }

    #[test]
    fn test_arch() {
        let arch = self::arch();
        println!("arch: {}", arch.to_string_lossy());
        assert!(!arch.is_empty());
    }
}
