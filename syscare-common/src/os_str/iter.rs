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

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as StdOsStrExt};

use super::{
    pattern::{Pattern, Searcher},
    utf8,
};

/* CharByteIndices */
pub struct CharIndices<'a> {
    pub(crate) char_bytes: &'a [u8],
    pub(crate) front_idx: usize,
    pub(crate) back_idx: usize,
}

impl Iterator for CharIndices<'_> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let char_bytes = &self.char_bytes[self.front_idx..];
        if char_bytes.is_empty() {
            return None;
        }

        match utf8::next_valid_char(char_bytes) {
            Some((len, c)) => {
                let result = (self.front_idx, c);
                self.front_idx += len;

                Some(result)
            }
            None => {
                // Unable to parse utf-8 char, fallback to byte
                let result = (self.front_idx, char::from(char_bytes[0]));
                self.front_idx += 1;

                Some(result)
            }
        }
    }
}

impl DoubleEndedIterator for CharIndices<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let char_bytes = &self.char_bytes[..self.back_idx];
        if char_bytes.is_empty() {
            return None;
        }

        match utf8::next_back_valid_char(char_bytes) {
            Some((len, c)) => {
                self.back_idx -= len;
                Some((self.back_idx, c))
            }
            None => {
                // Unable to parse utf-8 char, fallback to byte
                self.back_idx -= 1;
                Some((
                    self.back_idx,
                    char_bytes
                        .last()
                        .map(|b| char::from(*b))
                        .unwrap_or_default(),
                ))
            }
        }
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
        return Some(OsStr::from_bytes(&haystack[self.position..]));
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
