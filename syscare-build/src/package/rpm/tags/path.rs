// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RpmPath {
    Directory(PathBuf),
    File(PathBuf),
}

impl std::fmt::Display for RpmPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpmPath::Directory(path) => f.write_fmt(format_args!("%dir {}", path.display())),
            RpmPath::File(path) => f.write_fmt(format_args!("{}", path.display())),
        }
    }
}

#[test]
fn test() {
    let dir = RpmPath::Directory(PathBuf::from("/test/path"));
    println!("dir:\n{}\n", dir);
    assert_eq!(dir.to_string(), "%dir /test/path");

    let file = RpmPath::File(PathBuf::from("/test/path"));
    println!("file:\n{}\n", file);
    assert_eq!(file.to_string(), "/test/path");
}
