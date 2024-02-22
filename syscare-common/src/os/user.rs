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
use std::path::Path;

use lazy_static::*;
use nix::unistd::{getuid, User};

use crate::util::c_str::CStrExt;

#[inline(always)]
fn info() -> &'static User {
    lazy_static! {
        static ref USER: User = User::from_uid(getuid())
            .expect("Failed to read user info")
            .unwrap();
    }
    &USER
}

pub fn name() -> &'static str {
    info().name.as_str()
}

pub fn passwd() -> &'static OsStr {
    info().passwd.as_os_str()
}

pub fn id() -> u32 {
    info().uid.as_raw()
}

pub fn gid() -> u32 {
    info().gid.as_raw()
}

pub fn gecos() -> &'static OsStr {
    info().gecos.as_os_str()
}

pub fn home() -> &'static Path {
    info().dir.as_path()
}

pub fn shell() -> &'static Path {
    info().shell.as_path()
}

#[test]
fn test() {
    println!("name:   {}", name());
    assert!(!name().is_empty());

    println!("passwd: {}", passwd().to_string_lossy());
    assert!(!passwd().is_empty());

    println!("id:     {}", id());
    assert!(id() > 0);

    println!("gid:    {}", gid());
    assert!(gid() > 0);

    println!("gecos:  {}", gecos().to_string_lossy());

    println!("home:   {}", home().display());
    assert!(home().exists());

    println!("shell:  {}", shell().display());
    assert!(shell().exists());
}
