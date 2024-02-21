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
    convert::TryFrom,
    ffi::{OsStr, OsString},
    fs::File,
    io::BufReader,
};

use anyhow::Result;

use crate::util::{
    fs,
    os_line::{BufReadOsLines, OsLines},
    os_str::OsStrExt,
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
        let values = value.split_whitespace().collect::<Vec<_>>();
        let parse_value = |value: Option<&&OsStr>| -> OsString {
            value.map(|s| s.to_os_string()).unwrap_or_default()
        };

        Ok(Self {
            address: parse_value(values.get(0)),
            permission: parse_value(values.get(1)),
            offset: parse_value(values.get(2)),
            dev: parse_value(values.get(3)),
            inode: parse_value(values.get(4)),
            path_name: parse_value(values.get(5)),
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
