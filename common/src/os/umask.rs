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

use nix::sys::stat::{umask, Mode};

pub fn set_umask(mode: u32) -> u32 {
    umask(Mode::from_bits_truncate(mode)).bits()
}

#[test]
fn test() {
    use std::{fs, fs::File, os::unix::fs::PermissionsExt};

    const FILE_PATH: &str = "/tmp/umask_test";
    const UMASK1: u32 = 0o077; // 10600
    const UMASK2: u32 = 0o022; // 10644

    fs::remove_file(FILE_PATH).ok();

    println!("Testing umask {:03o}...", UMASK1);
    set_umask(UMASK1);
    let file1 = File::create(FILE_PATH).expect("Failed to create file");
    let perm1 = file1
        .metadata()
        .map(|s| s.permissions())
        .expect("Failed to read file permission");

    println!("umask: {:03o}, perm: {:05o}", UMASK1, perm1.mode());

    drop(file1);
    fs::remove_file(FILE_PATH).ok();

    println!("Testing umask {:03o}...", UMASK2);
    set_umask(UMASK2);
    let file2 = File::create(FILE_PATH).expect("Failed to create file");
    let perm2 = file2
        .metadata()
        .map(|s| s.permissions())
        .expect("Failed to read file permission");

    println!("umask: {:03o}, perm: {:05o}", UMASK2, perm2.mode());

    drop(file2);
    fs::remove_file(FILE_PATH).ok();
}
