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

/* CharByte */

use std::{ffi::OsString, iter::FromIterator, ops::Deref, os::unix::ffi::OsStringExt};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharByte {
    Byte(char), // save u8, but use char as storage
    Char(char),
}

impl CharByte {
    pub fn as_char(&self) -> &char {
        match self {
            CharByte::Char(c) => c,
            CharByte::Byte(b) => b,
        }
    }

    pub fn into_char(self) -> char {
        match self {
            CharByte::Char(c) => c,
            CharByte::Byte(b) => b,
        }
    }
}

impl Default for CharByte {
    fn default() -> Self {
        CharByte::from(0)
    }
}

impl From<u8> for CharByte {
    fn from(value: u8) -> Self {
        CharByte::Byte(char::from(value))
    }
}

impl From<char> for CharByte {
    fn from(value: char) -> Self {
        CharByte::Char(value)
    }
}

impl PartialEq<char> for CharByte {
    fn eq(&self, other: &char) -> bool {
        self.as_char().eq(other)
    }
}

impl PartialEq<CharByte> for char {
    fn eq(&self, other: &CharByte) -> bool {
        self.eq(other.as_char())
    }
}

impl PartialOrd<char> for CharByte {
    fn partial_cmp(&self, other: &char) -> Option<std::cmp::Ordering> {
        self.as_char().partial_cmp(other)
    }
}

impl PartialOrd<CharByte> for char {
    fn partial_cmp(&self, other: &CharByte) -> Option<std::cmp::Ordering> {
        self.partial_cmp(other.as_char())
    }
}

impl FromIterator<CharByte> for OsString {
    fn from_iter<T: IntoIterator<Item = CharByte>>(iter: T) -> Self {
        let buf = iter.into_iter().fold(Vec::new(), |mut buf, char_byte| {
            match char_byte {
                CharByte::Char(c) => buf.extend(c.to_string().as_bytes()),
                CharByte::Byte(b) => buf.push(b as u8),
            }
            buf
        });
        OsString::from_vec(buf)
    }
}

impl Deref for CharByte {
    type Target = char;

    fn deref(&self) -> &Self::Target {
        self.as_char()
    }
}

impl std::fmt::Debug for CharByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharByte::Char(c) => write!(f, "{}", c),
            CharByte::Byte(b) => write!(f, "\\x{:X}", *b as u8),
        }
    }
}

impl std::fmt::Display for CharByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
