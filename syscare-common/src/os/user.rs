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

use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

use nix::unistd;

#[inline(always)]
fn userinfo() -> unistd::User {
    unistd::User::from_uid(unistd::getuid())
        .expect("Failed to get user infomation")
        .expect("Invalid user id")
}

pub fn uid() -> u32 {
    unistd::getuid().as_raw()
}

pub fn gid() -> u32 {
    unistd::getgid().as_raw()
}

pub fn name() -> String {
    self::userinfo().name
}

pub fn passwd() -> OsString {
    OsString::from_vec(self::userinfo().passwd.into_bytes())
}

pub fn gecos() -> OsString {
    OsString::from_vec(self::userinfo().gecos.into_bytes())
}

pub fn home() -> PathBuf {
    self::userinfo().dir
}

pub fn shell() -> PathBuf {
    self::userinfo().shell
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_uid() {
        let uid = self::uid();
        println!("uid: {}", uid);
        assert!(uid < u32::MAX);
    }

    #[test]
    fn test_gid() {
        let gid = self::gid();
        println!("gid: {}", gid);
        assert!(gid < u32::MAX);
    }

    #[test]
    fn test_name() {
        let name = self::name();
        println!("name: {}", name);
        assert!(!name.is_empty());
    }

    #[test]
    fn test_passwd() {
        let passwd = self::passwd();
        println!("passwd: {}", passwd.to_string_lossy());
        assert!(!passwd.is_empty());
    }

    #[test]
    fn test_gecos() {
        let gecos = self::gecos();
        println!("gecos: {}", gecos.to_string_lossy());
    }

    #[test]
    fn test_home() {
        let home = self::home();
        println!("home: {}", home.display());
        assert!(home.exists());
    }

    #[test]
    fn test_shell() {
        let shell = self::shell();
        println!("shell:  {}", shell.display());
        assert!(shell.exists());
    }
}
