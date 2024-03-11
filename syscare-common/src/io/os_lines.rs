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

use std::{ffi::OsString, io::BufRead, os::unix::prelude::OsStringExt};

pub struct OsLines<R> {
    buf: R,
}

impl<R: BufRead> Iterator for OsLines<R> {
    type Item = std::io::Result<OsString>;

    fn next(&mut self) -> Option<Self::Item> {
        const CHAR_LF: [u8; 1] = [b'\n'];
        const CHAR_CR: [u8; 1] = [b'\r'];

        let mut buf = Vec::new();
        match self.buf.read_until(CHAR_LF[0], &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                // Drop "\n" or "\r\n" on the buf tail
                if buf.ends_with(&CHAR_LF) {
                    buf.pop();
                    if buf.ends_with(&CHAR_CR) {
                        buf.pop();
                    }
                }
                buf.shrink_to_fit();
                Some(Ok(OsString::from_vec(buf)))
            }
            Err(_) => Some(self.buf.read_to_end(&mut buf).map(|_| {
                buf.shrink_to_fit();
                OsString::from_vec(buf)
            })),
        }
    }
}

impl<R: BufRead> From<R> for OsLines<R> {
    fn from(buf: R) -> Self {
        Self { buf }
    }
}

pub trait BufReadOsLines: BufRead {
    fn os_lines(self) -> OsLines<Self>
    where
        Self: Sized,
    {
        OsLines::from(self)
    }
}

impl<R: BufRead> BufReadOsLines for R {}

#[test]
fn test() {
    use crate::fs;
    use std::io::BufReader;

    let buf_reader =
        BufReader::new(fs::open_file("/proc/self/cmdline").expect("Failed to open procfs"));
    let lines = buf_reader.lines();
    for str in lines.flatten() {
        println!("{}", str);
        assert!(!str.is_empty());
    }

    let buf_reader =
        BufReader::new(fs::open_file("/proc/self/cmdline").expect("Failed to open procfs"));
    let os_lines = OsLines::from(buf_reader);
    for str in os_lines.flatten() {
        println!("{}", str.to_string_lossy());
        assert!(!str.is_empty());
    }
}
