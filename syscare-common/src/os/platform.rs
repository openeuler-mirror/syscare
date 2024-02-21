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

use lazy_static::lazy_static;
use nix::sys::utsname::{uname, UtsName};

#[inline(always)]
fn info() -> &'static UtsName {
    lazy_static! {
        static ref PLATFORM_INFO: UtsName = uname().expect("Failed to get uname");
    }
    &PLATFORM_INFO
}

pub fn hostname() -> &'static OsStr {
    info().nodename()
}

pub fn sysname() -> &'static OsStr {
    info().sysname()
}

pub fn release() -> &'static OsStr {
    info().release()
}

pub fn version() -> &'static OsStr {
    info().version()
}

pub fn arch() -> &'static OsStr {
    info().machine()
}

#[test]
fn test() {
    let sysname = sysname();
    let hostname = hostname();
    let release = release();
    let version = version();
    let arch = arch();

    println!("sysname:  {}", sysname.to_string_lossy());
    assert!(!sysname.is_empty());

    println!("hostname: {}", hostname.to_string_lossy());
    assert!(!hostname.is_empty());

    println!("release:  {}", release.to_string_lossy());
    assert!(!release.is_empty());

    println!("version:  {}", version.to_string_lossy());
    assert!(!version.is_empty());

    println!("arch:     {}", arch.to_string_lossy());
    assert!(!arch.is_empty());
}
