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

use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use super::{
    char_byte::CharByte,
    pattern::{Pattern, Searcher},
    utf8,
};

/* Chars */
pub struct Chars<'a> {
    pub(crate) char_bytes: &'a [u8],
    pub(crate) front_idx: usize,
    pub(crate) back_idx: usize,
}

impl Iterator for Chars<'_> {
    type Item = CharByte;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_idx >= self.char_bytes.len() {
            return None;
        }

        let char_bytes = &self.char_bytes[self.front_idx..];
        if let Some((len, c)) = utf8::next_valid_char(char_bytes) {
            self.front_idx += len;
            return Some(CharByte::from(c));
        }

        // Unable to parse utf-8 char, fallback to byte
        self.front_idx += 1;
        Some(CharByte::from(char_bytes[0]))
    }
}

impl DoubleEndedIterator for Chars<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.back_idx == 0 {
            return None;
        }

        let char_bytes = &self.char_bytes[..self.back_idx];
        if let Some((len, c)) = utf8::next_back_valid_char(char_bytes) {
            self.back_idx -= len;
            return Some(CharByte::from(c));
        }

        // Unable to parse utf-8 char, fallback to byte
        self.back_idx -= 1;
        char_bytes.last().map(|&b| CharByte::from(b))
    }
}

/* CharByteIndices */
pub struct CharIndices<'a> {
    pub(crate) char_bytes: &'a [u8],
    pub(crate) front_idx: usize,
    pub(crate) back_idx: usize,
}

impl Iterator for CharIndices<'_> {
    type Item = (usize, CharByte);

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_idx >= self.char_bytes.len() {
            return None;
        }

        let char_bytes = &self.char_bytes[self.front_idx..];
        if let Some((len, c)) = utf8::next_valid_char(char_bytes) {
            let result = (self.front_idx, CharByte::from(c));
            self.front_idx += len;
            return Some(result);
        }

        // Unable to parse utf-8 char, fallback to byte
        let result = (self.front_idx, CharByte::from(char_bytes[0]));
        self.front_idx += 1;
        Some(result)
    }
}

impl DoubleEndedIterator for CharIndices<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.back_idx == 0 {
            return None;
        }

        let char_bytes = &self.char_bytes[..self.back_idx];
        if let Some((len, c)) = utf8::next_back_valid_char(char_bytes) {
            self.back_idx -= len;
            return Some((self.back_idx, CharByte::from(c)));
        }

        // Unable to parse utf-8 char, fallback to byte
        self.back_idx -= 1;
        char_bytes
            .last()
            .map(|&b| (self.back_idx, CharByte::from(b)))
    }
}

/* Split */
pub struct SplitImpl<'a, P: Pattern<'a>> {
    pub(crate) position: usize,
    pub(crate) searcher: P::Searcher,
    pub(crate) finished: bool,
    pub(crate) allow_tailing_empty: bool,
}

impl<'a, P: Pattern<'a>> SplitImpl<'a, P> {
    #[inline]
    fn next(&mut self) -> Option<&'a OsStr> {
        if self.finished {
            return None;
        }

        let haystack = self.searcher.haystack();
        match self.searcher.next_match() {
            Some((start, end)) => {
                let elt = OsStr::from_bytes(&haystack[self.position..start]);
                self.position = end;
                Some(elt)
            }
            None => self.get_end(),
        }
    }

    #[inline]
    fn next_inclusive(&mut self) -> Option<&'a OsStr> {
        if self.finished {
            return None;
        }

        let haystack = self.searcher.haystack();
        match self.searcher.next_match() {
            Some((_, end)) => {
                let elt = OsStr::from_bytes(&haystack[self.position..end]);
                self.position = end;
                Some(elt)
            }
            None => self.get_end(),
        }
    }

    #[inline]
    fn get_end(&mut self) -> Option<&'a OsStr> {
        let haystack = self.searcher.haystack();
        if !self.allow_tailing_empty && (haystack.len() - self.position) == 0 {
            return None;
        }

        self.finished = true;
        Some(OsStr::from_bytes(&haystack[self.position..]))
    }
}

/* Split */
pub struct Split<'a, P: Pattern<'a>>(pub(crate) SplitImpl<'a, P>);

impl<'a, P: Pattern<'a>> Iterator for Split<'a, P> {
    type Item = &'a OsStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/* SplitInclusive */
pub struct SplitInclusive<'a, P: Pattern<'a>>(pub(crate) SplitImpl<'a, P>);

impl<'a, P: Pattern<'a>> Iterator for SplitInclusive<'a, P> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_inclusive()
    }
}

/* Lines */
pub struct Lines<'a>(pub(crate) SplitInclusive<'a, char>);

impl<'a> Iterator for Lines<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
