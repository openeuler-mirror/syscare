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
    ffi::{CString, OsStr},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use nix::unistd::{getuid, Gid, Uid, User};

use crate::ffi::CStrExt;

fn info() -> &'static User {
    lazy_static! {
        static ref USER_INFO: User = User::from_uid(getuid())
            .unwrap_or_default()
            .unwrap_or(User {
                name: String::from("root"),
                passwd: CString::default(),
                uid: Uid::from_raw(0),
                gid: Gid::from_raw(0),
                gecos: CString::default(),
                dir: PathBuf::from("/root"),
                shell: PathBuf::from("/bin/sh"),
            });
    }
    &USER_INFO
}

pub fn name() -> &'static str {
    self::info().name.as_str()
}

pub fn passwd() -> &'static OsStr {
    self::info().passwd.as_os_str()
}

pub fn id() -> u32 {
    self::info().uid.as_raw()
}

pub fn gid() -> u32 {
    self::info().gid.as_raw()
}

pub fn gecos() -> &'static OsStr {
    self::info().gecos.as_os_str()
}

pub fn home() -> &'static Path {
    self::info().dir.as_path()
}

pub fn shell() -> &'static Path {
    self::info().shell.as_path()
}

#[test]
fn test() {
    println!("name:   {}", self::name());
    assert!(!self::name().is_empty());

    println!("passwd: {}", self::passwd().to_string_lossy());
    assert!(!self::passwd().is_empty());

    println!("id:     {}", self::id());
    assert!(id() < u32::MAX);

    println!("gid:    {}", self::gid());
    assert!(gid() < u32::MAX);

    println!("gecos:  {}", self::gecos().to_string_lossy());

    println!("home:   {}", self::home().display());
    assert!(self::home().exists());

    println!("shell:  {}", self::shell().display());
    assert!(self::home().exists());
}
