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

use std::{convert::TryFrom, ffi::OsString, fs::File, io::BufReader};

use anyhow::{ensure, Result};

use crate::{
    ffi::OsStrExt,
    fs,
    io::{BufReadOsLines, OsLines},
};

#[derive(Debug)]
pub struct ProcMap {
    pub address: OsString,
    pub permission: OsString,
    pub offset: OsString,
    pub dev: OsString,
    pub inode: OsString,
    pub path_name: OsString,
}

impl TryFrom<OsString> for ProcMap {
    type Error = anyhow::Error;

    fn try_from(value: OsString) -> std::result::Result<Self, Self::Error> {
        const MAP_FIELD_NUM: usize = 6;

        let fields = value.split_whitespace().collect::<Vec<_>>();
        ensure!(
            fields.len() == MAP_FIELD_NUM,
            "Failed to parse process mapping"
        );

        Ok(Self {
            address: fields[0].to_owned(),
            permission: fields[1].to_owned(),
            offset: fields[2].to_owned(),
            dev: fields[3].to_owned(),
            inode: fields[4].to_owned(),
            path_name: fields[5].to_owned(),
        })
    }
}

pub struct ProcMaps {
    lines: OsLines<BufReader<File>>,
}

impl ProcMaps {
    pub fn new(pid: i32) -> Result<Self> {
        let file_path = format!("/proc/{}/maps", pid);
        let lines = BufReader::new(fs::open_file(file_path)?).os_lines();

        Ok(Self { lines })
    }
}

impl Iterator for ProcMaps {
    type Item = ProcMap;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines
            .next()
            .and_then(Result::ok)
            .map(ProcMap::try_from)
            .and_then(Result::ok)
    }
}

#[test]
fn test() {
    use super::process;

    for map in ProcMaps::new(process::id()).expect("Failed to read proc maps") {
        println!("{:#?}", map);
    }
}
