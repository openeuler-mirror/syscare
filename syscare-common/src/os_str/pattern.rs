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
    ffi::{OsStr, OsString},
    os::unix::prelude::OsStrExt,
};

use super::CharIndices;

/* Searcher & ReverseSearcher */
pub enum SearchStep {
    Match(usize, usize), // (start index, end index)
    Reject(usize, usize),
    Done,
}

pub trait Searcher<'a> {
    fn haystack(&self) -> &'a [u8];

    fn next(&mut self) -> SearchStep;

    #[inline]
    fn next_match(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next() {
                SearchStep::Match(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }

    #[inline]
    fn next_reject(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next() {
                SearchStep::Reject(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }
}

pub trait ReverseSearcher<'a>: Searcher<'a> {
    fn next_back(&mut self) -> SearchStep;

    #[inline]
    fn next_match_back(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next_back() {
                SearchStep::Match(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }

    #[inline]
    fn next_reject_back(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next_back() {
                SearchStep::Reject(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }
}

/* CharLiteralSearcher */
pub struct CharLiteralSearcher<'a> {
    indices: CharIndices<'a>,
    literals: Vec<char>,
}

impl<'a> CharLiteralSearcher<'a> {
    pub fn new(haystack: &'a [u8], literals: Vec<char>) -> Self {
        Self {
            indices: CharIndices {
                char_bytes: haystack,
                front_idx: 0,
                back_idx: haystack.len(),
            },
            literals,
        }
    }
}

impl<'a> Searcher<'a> for CharLiteralSearcher<'a> {
    fn haystack(&self) -> &'a [u8] {
        self.indices.char_bytes
    }

    fn next(&mut self) -> SearchStep {
        match self.indices.next() {
            Some((char_idx, c)) => {
                let new_idx = char_idx + c.len_utf8();
                if self.literals.contains(&c) {
                    SearchStep::Match(char_idx, new_idx)
                } else {
                    SearchStep::Reject(char_idx, new_idx)
                }
            }
            None => SearchStep::Done,
        }
    }
}

impl<'a> ReverseSearcher<'a> for CharLiteralSearcher<'a> {
    fn next_back(&mut self) -> SearchStep {
        match self.indices.next_back() {
            Some((char_idx, c)) => {
                let new_idx = char_idx + c.len_utf8();
                if self.literals.contains(&c) {
                    SearchStep::Match(char_idx, new_idx)
                } else {
                    SearchStep::Reject(char_idx, new_idx)
                }
            }
            None => SearchStep::Done,
        }
    }
}

/* CharPredicateSearcher */
pub struct CharPredicateSearcher<'a, P> {
    indices: CharIndices<'a>,
    predicate: P,
}

impl<'a, P: FnMut(char) -> bool> CharPredicateSearcher<'a, P> {
    fn new(haystack: &'a [u8], predicate: P) -> Self {
        Self {
            indices: CharIndices {
                char_bytes: haystack,
                front_idx: 0,
                back_idx: haystack.len(),
            },
            predicate,
        }
    }
}

impl<'a, P: FnMut(char) -> bool> Searcher<'a> for CharPredicateSearcher<'a, P> {
    fn haystack(&self) -> &'a [u8] {
        self.indices.char_bytes
    }

    fn next(&mut self) -> SearchStep {
        match self.indices.next() {
            Some((char_idx, c)) => {
                let new_idx = char_idx + c.len_utf8();
                if (self.predicate)(c.into_char()) {
                    SearchStep::Match(char_idx, new_idx)
                } else {
                    SearchStep::Reject(char_idx, new_idx)
                }
            }
            None => SearchStep::Done,
        }
    }
}

impl<'a, P: FnMut(char) -> bool> ReverseSearcher<'a> for CharPredicateSearcher<'a, P> {
    fn next_back(&mut self) -> SearchStep {
        match self.indices.next_back() {
            Some((char_idx, c)) => {
                let new_idx = char_idx + c.len_utf8();
                if (self.predicate)(c.into_char()) {
                    SearchStep::Match(char_idx, new_idx)
                } else {
                    SearchStep::Reject(char_idx, new_idx)
                }
            }
            None => SearchStep::Done,
        }
    }
}

/* OsStrSearcher */
pub struct OsStrSearcher<'a, T> {
    indices: CharIndices<'a>,
    needle: T,
    match_fw: bool,
    match_bw: bool,
}

impl<'a, T> OsStrSearcher<'a, T> {
    fn new(haystack: &'a [u8], needle: T) -> Self {
        Self {
            indices: CharIndices {
                char_bytes: haystack,
                front_idx: 0,
                back_idx: haystack.len(),
            },
            needle,
            match_fw: false,
            match_bw: false,
        }
    }
}

impl<'a, T: AsRef<OsStr>> Searcher<'a> for OsStrSearcher<'a, T> {
    fn haystack(&self) -> &'a [u8] {
        self.indices.char_bytes
    }

    fn next(&mut self) -> SearchStep {
        let haystack: &mut CharIndices = &mut self.indices;
        let needle_bytes = self.needle.as_ref().as_bytes();
        let mut needle = CharIndices {
            char_bytes: needle_bytes,
            front_idx: 0,
            back_idx: needle_bytes.len(),
        };

        let front_idx = haystack.front_idx;

        if needle.char_bytes.is_empty() {
            // Case 1: Needle is empty, rejects every char and matches every empty string between them
            self.match_fw = !self.match_fw;
            if self.match_fw {
                return SearchStep::Match(front_idx, front_idx);
            }
            match haystack.next() {
                Some(_) => {
                    return SearchStep::Reject(front_idx, haystack.front_idx);
                }
                None => {
                    return SearchStep::Done;
                }
            }
        }

        // Compare every chars in needle with haystack in sequence
        for (_, needle_char) in needle.by_ref() {
            if let Some((_, haystack_char)) = haystack.next() {
                if haystack_char != needle_char {
                    // Case 2: Char mismatched, stop matching, reject matched chars
                    return SearchStep::Reject(front_idx, haystack.front_idx);
                }
                continue;
            }
            let bytes_left = haystack.char_bytes.len() - haystack.front_idx;
            if bytes_left != 0 {
                // Case 3: Haystack is empty, but needle has chars left, reject left chars in haystack
                return SearchStep::Reject(front_idx, front_idx + bytes_left);
            }
            // Case 4: Haystack has nothing left, search done
            return SearchStep::Done;
        }
        // Case 5: All chars in neelde are matched
        SearchStep::Match(front_idx, haystack.front_idx)
    }
}

impl<'a, T: AsRef<OsStr>> ReverseSearcher<'a> for OsStrSearcher<'a, T> {
    fn next_back(&mut self) -> SearchStep {
        let haystack = &mut self.indices;

        let needle_bytes = self.needle.as_ref().as_bytes();
        let mut needle = CharIndices {
            char_bytes: needle_bytes,
            front_idx: 0,
            back_idx: needle_bytes.len(),
        };

        let back_idx = haystack.back_idx;
        if needle.char_bytes.is_empty() {
            // Case 1: Needle is empty, rejects every char and matches every empty string between them
            self.match_bw = !self.match_bw;
            if self.match_bw {
                return SearchStep::Match(back_idx, back_idx);
            }
            match haystack.next_back() {
                Some(_) => {
                    return SearchStep::Reject(haystack.back_idx, back_idx);
                }
                None => {
                    return SearchStep::Done;
                }
            }
        }

        // Compare every chars in needle with haystack in sequence
        while let Some((_, needle_char)) = needle.next_back() {
            if let Some((_, haystack_char)) = haystack.next_back() {
                if haystack_char != needle_char {
                    // Case 2: Char mismatched, stop matching, reject matched chars
                    return SearchStep::Reject(haystack.back_idx, back_idx);
                }
                continue;
            }
            if haystack.back_idx != 0 {
                // Case 3: Haystack is empty, but needle has chars left, reject left chars in haystack
                return SearchStep::Reject(0, back_idx);
            }
            // Case 4: Haystack has nothing left, search done
            return SearchStep::Done;
        }
        // Case 5: All chars in neelde are matched
        SearchStep::Match(haystack.back_idx, back_idx)
    }
}

/* Pattern */
pub trait Pattern<'a>: Sized {
    type Searcher: Searcher<'a>;
    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher;
}

impl<'a> Pattern<'a> for char {
    type Searcher = CharLiteralSearcher<'a>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharLiteralSearcher::new(haystack, vec![self])
    }
}

impl<'a, const ARRAY_SIZE: usize> Pattern<'a> for [char; ARRAY_SIZE] {
    type Searcher = CharLiteralSearcher<'a>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharLiteralSearcher::new(haystack, self.to_vec())
    }
}

impl<'a, const ARRAY_SIZE: usize> Pattern<'a> for &[char; ARRAY_SIZE] {
    type Searcher = CharLiteralSearcher<'a>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharLiteralSearcher::new(haystack, self.to_vec())
    }
}

impl<'a> Pattern<'a> for &Vec<char> {
    type Searcher = CharLiteralSearcher<'a>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharLiteralSearcher::new(haystack, self.to_vec())
    }
}

impl<'a> Pattern<'a> for Vec<char> {
    type Searcher = CharLiteralSearcher<'a>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharLiteralSearcher::new(haystack, self)
    }
}

impl<'a, P: FnMut(char) -> bool> Pattern<'a> for P {
    type Searcher = CharPredicateSearcher<'a, P>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        CharPredicateSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for String {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for &'a String {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for &'a str {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for OsString {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for &'a OsString {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}

impl<'a> Pattern<'a> for &'a OsStr {
    type Searcher = OsStrSearcher<'a, Self>;

    fn into_searcher(self, haystack: &'a [u8]) -> Self::Searcher {
        OsStrSearcher::new(haystack, self)
    }
}
