// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
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
use std::path::{Path, PathBuf};

/* Contains */
pub trait OsStrContains
where
    Self: AsRef<OsStr>,
{
    fn contains<S: AsRef<[u8]>>(&self, other: S) -> bool {
        let needle = other.as_ref();

        std::os::unix::prelude::OsStrExt::as_bytes(self.as_ref())
            .windows(needle.len())
            .any(|window| window == needle)
    }
}

impl OsStrContains for OsStr {}
impl OsStrContains for OsString {}
impl OsStrContains for Path {}
impl OsStrContains for PathBuf {}

/* Concat */
pub trait OsStrConcat {
    fn concat<T: AsRef<OsStr>>(&mut self, s: T) -> &mut Self;
}

impl OsStrConcat for OsString {
    fn concat<T: AsRef<OsStr>>(&mut self, s: T) -> &mut Self {
        self.push(s);
        self
    }
}
