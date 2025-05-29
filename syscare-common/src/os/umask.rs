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

use nix::{libc::mode_t, sys::stat};

pub fn set_umask(mode: mode_t) -> mode_t {
    stat::umask(stat::Mode::from_bits_truncate(mode)).bits()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_set_umask() {
        use std::{fs, fs::File, os::unix::fs::PermissionsExt};

        let file_path = std::env::temp_dir().join("umask_test");
        const UMASK1: u32 = 0o077; // 10600
        const UMASK2: u32 = 0o022; // 10644

        fs::remove_file(&file_path).ok();

        println!("Testing umask {:03o}...", UMASK1);
        super::set_umask(UMASK1);
        let file1 = File::create(&file_path).expect("Failed to create file");
        let perm1 = file1
            .metadata()
            .map(|s| s.permissions())
            .expect("Failed to read file permission");
        println!("umask: {:03o}, perm: {:05o}", UMASK1, perm1.mode());
        assert_eq!(perm1.mode() & 0o777, 0o600);

        drop(file1);
        fs::remove_file(&file_path).expect("Failed to remove file");

        println!("Testing umask {:03o}...", UMASK2);
        super::set_umask(UMASK2);
        let file2 = File::create(&file_path).expect("Failed to create file");
        let perm2 = file2
            .metadata()
            .map(|s| s.permissions())
            .expect("Failed to read file permission");

        println!("umask: {:03o}, perm: {:05o}", UMASK2, perm2.mode());
        assert_eq!(perm2.mode() & 0o777, 0o644);

        drop(file2);
        fs::remove_file(&file_path).expect("Failed to remove file");
    }
}
