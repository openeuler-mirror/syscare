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

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::BufReader;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use anyhow::Result;

use crate::util::fs;
use crate::util::os_line::{BufReadOsLines, OsLines};
use crate::util::os_str::OsStrExt as OsStrUtil;

#[derive(Debug)]
pub struct MountInfo {
    pub mount_id: u32,
    pub parent_id: u32,
    pub device_id: OsString,
    pub root: PathBuf,
    pub mount_point: PathBuf,
    pub mount_opts: Vec<OsString>,
    pub optional: Vec<OsString>,
    pub filesystem: OsString,
    pub mount_source: PathBuf,
    pub super_opts: Vec<OsString>,
}

struct MountInfoParser<'a> {
    data: &'a [u8],
    num: usize,
    pos: usize,
}

impl<'a> Iterator for MountInfoParser<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        const OPTION_INDEX: usize = 6;
        const NORMAL_SPLITTER: char = ' ';
        const OPTION_SPLITTER: char = '-';

        let data = &self.data[self.pos..];
        let new_str = OsStr::from_bytes(data);
        if new_str.is_empty() {
            return None;
        }

        for char_indices in new_str.char_indices() {
            let pattern;
            let skip_len;

            match self.num {
                OPTION_INDEX => {
                    pattern = OPTION_SPLITTER;
                    skip_len = 2;
                }
                _ => {
                    pattern = NORMAL_SPLITTER;
                    skip_len = 1;
                }
            };
            if char_indices.char() == pattern {
                self.num += 1;
                self.pos += char_indices.index() + skip_len;

                return Some(OsStr::from_bytes(&data[..char_indices.index()]));
            }
        }

        self.pos = self.data.len() - 1;
        Some(OsStr::from_bytes(data))
    }
}

pub struct Mounts {
    lines: OsLines<BufReader<File>>,
}

impl Mounts {
    pub fn new() -> Result<Self> {
        const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

        Ok(Self {
            lines: BufReader::new(fs::open_file(MOUNTINFO_PATH)?).os_lines(),
        })
    }
}

impl Mounts {
    fn parse_line(str: OsString) -> Option<MountInfo> {
        const VALUE_SPLITTER: char = ',';

        let str_bytes = str.into_vec();
        let mut iter = MountInfoParser {
            data: &str_bytes,
            num: 0,
            pos: 0,
        };

        Some(MountInfo {
            mount_id: iter.next()?.to_string_lossy().parse::<u32>().ok()?,
            parent_id: iter.next()?.to_string_lossy().parse::<u32>().ok()?,
            device_id: iter.next()?.to_os_string(),
            root: PathBuf::from(iter.next()?),
            mount_point: PathBuf::from(iter.next()?),
            mount_opts: iter
                .next()?
                .split(VALUE_SPLITTER)
                .map(OsStrUtil::trim)
                .map(OsString::from)
                .collect::<Vec<_>>(),
            optional: iter
                .next()?
                .split_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>(),
            filesystem: iter.next()?.to_os_string(),
            mount_source: PathBuf::from(iter.next()?),
            super_opts: iter
                .next()?
                .split(VALUE_SPLITTER)
                .map(OsStrUtil::trim)
                .map(OsString::from)
                .collect::<Vec<_>>(),
        })
    }
}

impl Iterator for Mounts {
    type Item = MountInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines.next()?.ok().and_then(Self::parse_line)
    }
}

#[test]
fn test() {
    let mount_info = Mounts::new().expect("Failed to read mount info");
    for mount in mount_info {
        println!();
        println!("{:#?}", mount);
        assert!(mount.mount_point.exists())
    }
}
