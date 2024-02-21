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

use std::path::Path;

use sha2::Digest;
use sha2::Sha256;

use super::fs;

pub fn bytes<S: AsRef<[u8]>>(bytes: S) -> String {
    let mut hasher = Sha256::new();
    hasher.input(bytes);

    format!("{:#x}", hasher.result())
}

pub fn file<P: AsRef<Path>>(file: P) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    hasher.input(fs::read(file)?);

    Ok(format!("{:#x}", hasher.result()))
}

pub fn file_list<I, P>(file_list: I) -> std::io::Result<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut hasher = Sha256::new();
    for file in file_list {
        hasher.input(fs::read(file)?);
    }

    Ok(format!("{:#x}", hasher.result()))
}

pub fn dir<P: AsRef<Path>>(directory: P) -> std::io::Result<String> {
    file_list(fs::list_files(
        directory,
        fs::TraverseOptions { recursive: true },
    )?)
}
