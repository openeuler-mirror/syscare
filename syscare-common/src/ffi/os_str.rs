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
    ffi::{CString, NulError, OsStr, OsString},
    iter::{Filter, Map},
    os::unix::{ffi::OsStringExt as StdOsStringExt, prelude::OsStrExt as StdOsStrExt},
    path::{Path, PathBuf},
};

use crate::os_str::{
    pattern::{Pattern, ReverseSearcher, SearchStep, Searcher},
    CharIndices, Lines, Split, SplitImpl, SplitInclusive,
};

pub type SplitFn = fn(char) -> bool;
pub type FilterFn = fn(&&OsStr) -> bool;
pub type MapFn = fn(&OsStr) -> &OsStr;

/* OsStrExt */
pub trait OsStrExt: AsRef<OsStr> {
    fn is_char_boundary(&self, index: usize) -> bool {
        if index == 0 {
            return true;
        }

        let haystack = self.as_ref().as_bytes();
        match haystack.get(index) {
            Some(&b) => {
                // This is bit magic equivalent to: b < 128 || b >= 192
                b as i8 >= -0x40
            }
            None => index == haystack.len(),
        }
    }

    fn char_indices(&self) -> CharIndices<'_> {
        let haystack = self.as_ref().as_bytes();

        CharIndices {
            char_bytes: haystack,
            front_idx: 0,
            back_idx: haystack.len(),
        }
    }

    fn find<'a, P: Pattern<'a>>(&'a self, pat: P) -> Option<usize> {
        pat.into_searcher(self.as_ref().as_bytes())
            .next_match()
            .map(|(i, _)| i)
    }

    fn contains<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        self.find(pat).is_some()
    }

    fn starts_with<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        let haystack = self.as_ref().as_bytes();

        matches!(pat.into_searcher(haystack).next(), SearchStep::Match(0, _))
    }

    fn ends_with<'a, P>(&'a self, pat: P) -> bool
    where
        P: Pattern<'a>,
        P::Searcher: ReverseSearcher<'a>,
    {
        let haystack = self.as_ref().as_bytes();

        matches!(
            pat.into_searcher(haystack).next_back(),
            SearchStep::Match(_, j) if haystack.len() == j
        )
    }

    fn strip_prefix<'a, P: Pattern<'a>>(&'a self, pat: P) -> Option<&'a OsStr> {
        let haystack = self.as_ref().as_bytes();

        match pat.into_searcher(haystack).next() {
            SearchStep::Match(start, end) => {
                debug_assert_eq!(start, 0);
                Some(OsStr::from_bytes(&haystack[end..]))
            }
            _ => None,
        }
    }

    fn strip_suffix<'a, P>(&'a self, pat: P) -> Option<&'a OsStr>
    where
        P: Pattern<'a>,
        P::Searcher: ReverseSearcher<'a>,
    {
        let haystack = self.as_ref().as_bytes();

        match pat.into_searcher(haystack).next_back() {
            SearchStep::Match(start, end) => {
                debug_assert_eq!(end, haystack.len());
                Some(OsStr::from_bytes(&haystack[..start]))
            }
            _ => None,
        }
    }

    fn split<'a, P: Pattern<'a>>(&'a self, pat: P) -> Split<P> {
        let haystack = self.as_ref().as_bytes();

        Split(SplitImpl {
            position: 0,
            searcher: pat.into_searcher(haystack),
            allow_tailing_empty: true,
            finished: false,
        })
    }

    fn split_inclusive<'a, P: Pattern<'a>>(&'a self, pat: P) -> SplitInclusive<'a, P> {
        let haystack = self.as_ref().as_bytes();

        SplitInclusive(SplitImpl {
            position: 0,
            searcher: pat.into_searcher(haystack),
            allow_tailing_empty: false,
            finished: false,
        })
    }

    fn split_whitespace(&self) -> Filter<Split<SplitFn>, FilterFn> {
        self.split(char::is_whitespace as SplitFn)
            .filter(|s| !s.is_empty())
    }

    fn split_at(&self, mid: usize) -> (&OsStr, &OsStr) {
        if !self.is_char_boundary(mid) {
            panic!("Failed to slice osstring");
        }

        let (lhs, rhs) = self.as_ref().as_bytes().split_at(mid);
        (OsStr::from_bytes(lhs), OsStr::from_bytes(rhs))
    }

    fn trim_start_matches<'a, P: Pattern<'a>>(&'a self, pat: P) -> &'a OsStr {
        let haystack = self.as_ref().as_bytes();
        let start = match pat.into_searcher(haystack).next_reject() {
            Some((a, _)) => a,
            None => haystack.len(),
        };

        OsStr::from_bytes(&haystack[start..])
    }

    fn trim_end_matches<'a, P>(&'a self, pat: P) -> &'a OsStr
    where
        P: Pattern<'a>,
        P::Searcher: ReverseSearcher<'a>,
    {
        let haystack = self.as_ref().as_bytes();
        let end = match pat.into_searcher(haystack).next_reject_back() {
            Some((_, b)) => b,
            None => 0,
        };

        OsStr::from_bytes(&haystack[..end])
    }

    fn trim_matches<'a, P>(&'a self, pat: P) -> &'a OsStr
    where
        P: Pattern<'a>,
        P::Searcher: ReverseSearcher<'a>,
    {
        let haystack = self.as_ref().as_bytes();
        let mut start = 0;
        let mut end = 0;
        let mut matcher = pat.into_searcher(haystack);

        if let Some((a, b)) = matcher.next_reject() {
            start = a;
            end = b;
        }

        if let Some((_, b)) = matcher.next_reject_back() {
            end = b;
        }

        OsStr::from_bytes(&haystack[start..end])
    }

    fn trim(&self) -> &OsStr {
        self.trim_matches(char::is_whitespace)
    }

    fn trim_start(&self) -> &OsStr {
        self.trim_start_matches(char::is_whitespace)
    }

    fn trim_end(&self) -> &OsStr {
        self.trim_end_matches(char::is_whitespace)
    }

    fn lines(&self) -> Map<Lines<'_>, MapFn> {
        Lines(self.split_inclusive('\n')).map(|mut line| {
            if let Some(new_line) = line.strip_suffix('\n') {
                line = new_line;
            };
            if let Some(new_line) = line.strip_suffix('\r') {
                line = new_line;
            };
            line
        })
    }

    fn replace<'a, P: Pattern<'a>, S: AsRef<OsStr>>(&'a self, from: P, to: S) -> OsString {
        let haystack = self.as_ref().as_bytes();
        let replace = to.as_ref().as_bytes();

        let mut buf = Vec::new();
        let mut searcher = from.into_searcher(haystack);
        let mut last_idx = 0;

        while let Some((start, end)) = searcher.next_match() {
            buf.extend(&haystack[last_idx..start]);
            buf.extend(replace);
            last_idx = end;
        }
        buf.extend(&haystack[last_idx..]);

        OsString::from_vec(buf)
    }

    fn to_cstring(&self) -> Result<CString, NulError> {
        let haystack = self.as_ref().as_bytes();

        CString::new(haystack)
    }
}

impl OsStrExt for OsStr {}
impl OsStrExt for Path {}

impl OsStrExt for OsString {}
impl OsStrExt for PathBuf {}

#[test]
fn test() {
    const PATTERN0: &str = "T";
    const PATTERN1: &str = "fox";
    const PATTERN2: &str = "slow";
    const PATTERN3: &str = "\t";
    const PATTERN4: char = '\n';
    const PATTERN5: char = '\r';
    const PATTERN6: char = '\0';
    const PATTERN7: &str = "";
    const PATTERN8: fn(char) -> bool = |c: char| c.is_ascii_control();
    const PATTERN9: fn(char) -> bool = char::is_whitespace;

    let orig_str = "\r\n\tThe\tquick\tbrown\tfox\tjumps\tover\ta\tlazy\tdog\x01\u{1F600}\r\n";
    let test_str = OsStr::new(orig_str);

    println!("Testing OsStrExt::is_char_boundary()...");
    for index in 0..orig_str.as_bytes().len() {
        assert_eq!(
            orig_str.is_char_boundary(index),
            test_str.is_char_boundary(index)
        );
    }

    println!("Testing OsStrExt::char_indices()...");
    assert_eq!(
        orig_str.char_indices().collect::<Vec<_>>(),
        test_str.char_indices().collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.char_indices().rev().collect::<Vec<_>>(),
        test_str.char_indices().rev().collect::<Vec<_>>()
    );

    println!("Testing OsStrExt::find()...");
    assert_eq!(orig_str.find(PATTERN0), test_str.find(PATTERN0));
    assert_eq!(orig_str.find(PATTERN1), test_str.find(PATTERN1));
    assert_eq!(orig_str.find(PATTERN2), test_str.find(PATTERN2));
    assert_eq!(orig_str.find(PATTERN3), test_str.find(PATTERN3));
    assert_eq!(orig_str.find(PATTERN4), test_str.find(PATTERN4));
    assert_eq!(orig_str.find(PATTERN5), test_str.find(PATTERN5));
    assert_eq!(orig_str.find(PATTERN6), test_str.find(PATTERN6));
    assert_eq!(orig_str.find(PATTERN7), test_str.find(PATTERN7));
    assert_eq!(orig_str.find(PATTERN8), test_str.find(PATTERN8));
    assert_eq!(orig_str.find(PATTERN9), test_str.find(PATTERN9));

    println!("Testing OsStrExt::contains()...");
    assert_eq!(orig_str.contains(PATTERN0), test_str.contains(PATTERN0));
    assert_eq!(orig_str.contains(PATTERN1), test_str.contains(PATTERN1));
    assert_eq!(orig_str.contains(PATTERN2), test_str.contains(PATTERN2));
    assert_eq!(orig_str.contains(PATTERN3), test_str.contains(PATTERN3));
    assert_eq!(orig_str.contains(PATTERN4), test_str.contains(PATTERN4));
    assert_eq!(orig_str.contains(PATTERN5), test_str.contains(PATTERN5));
    assert_eq!(orig_str.contains(PATTERN6), test_str.contains(PATTERN6));
    assert_eq!(orig_str.contains(PATTERN7), test_str.contains(PATTERN7));
    assert_eq!(orig_str.contains(PATTERN8), test_str.contains(PATTERN8));
    assert_eq!(orig_str.contains(PATTERN9), test_str.contains(PATTERN9));

    println!("Testing OsStrExt::starts_with()...");
    assert_eq!(
        orig_str.starts_with(PATTERN0),
        test_str.starts_with(PATTERN0)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN1),
        test_str.starts_with(PATTERN1)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN2),
        test_str.starts_with(PATTERN2)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN3),
        test_str.starts_with(PATTERN3)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN4),
        test_str.starts_with(PATTERN4)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN5),
        test_str.starts_with(PATTERN5)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN6),
        test_str.starts_with(PATTERN6)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN7),
        test_str.starts_with(PATTERN7)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN8),
        test_str.starts_with(PATTERN8)
    );
    assert_eq!(
        orig_str.starts_with(PATTERN9),
        test_str.starts_with(PATTERN9)
    );

    println!("Testing OsStrExt::ends_with()...");
    assert_eq!(orig_str.ends_with(PATTERN0), test_str.ends_with(PATTERN0));
    assert_eq!(orig_str.ends_with(PATTERN1), test_str.ends_with(PATTERN1));
    assert_eq!(orig_str.ends_with(PATTERN2), test_str.ends_with(PATTERN2));
    assert_eq!(orig_str.ends_with(PATTERN3), test_str.ends_with(PATTERN3));
    assert_eq!(orig_str.ends_with(PATTERN4), test_str.ends_with(PATTERN4));
    assert_eq!(orig_str.ends_with(PATTERN5), test_str.ends_with(PATTERN5));
    assert_eq!(orig_str.ends_with(PATTERN6), test_str.ends_with(PATTERN6));
    assert_eq!(orig_str.ends_with(PATTERN7), test_str.ends_with(PATTERN7));
    assert_eq!(orig_str.ends_with(PATTERN8), test_str.ends_with(PATTERN8));
    assert_eq!(orig_str.ends_with(PATTERN9), test_str.ends_with(PATTERN9));

    println!("Testing OsStrExt::strip_prefix()...");
    assert_eq!(
        orig_str.strip_prefix(PATTERN0).map(OsStr::new),
        test_str.strip_prefix(PATTERN0)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN1).map(OsStr::new),
        test_str.strip_prefix(PATTERN1)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN2).map(OsStr::new),
        test_str.strip_prefix(PATTERN2)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN3).map(OsStr::new),
        test_str.strip_prefix(PATTERN3)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN4).map(OsStr::new),
        test_str.strip_prefix(PATTERN4)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN5).map(OsStr::new),
        test_str.strip_prefix(PATTERN5)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN6).map(OsStr::new),
        test_str.strip_prefix(PATTERN6)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN7).map(OsStr::new),
        test_str.strip_prefix(PATTERN7)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN8).map(OsStr::new),
        test_str.strip_prefix(PATTERN8)
    );
    assert_eq!(
        orig_str.strip_prefix(PATTERN9).map(OsStr::new),
        test_str.strip_prefix(PATTERN9)
    );

    println!("Testing OsStrExt::strip_suffix()...");
    assert_eq!(
        orig_str.strip_suffix(PATTERN0).map(OsStr::new),
        test_str.strip_suffix(PATTERN0)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN1).map(OsStr::new),
        test_str.strip_suffix(PATTERN1)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN2).map(OsStr::new),
        test_str.strip_suffix(PATTERN2)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN3).map(OsStr::new),
        test_str.strip_suffix(PATTERN3)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN4).map(OsStr::new),
        test_str.strip_suffix(PATTERN4)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN5).map(OsStr::new),
        test_str.strip_suffix(PATTERN5)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN6).map(OsStr::new),
        test_str.strip_suffix(PATTERN6)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN7).map(OsStr::new),
        test_str.strip_suffix(PATTERN7)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN8).map(OsStr::new),
        test_str.strip_suffix(PATTERN8)
    );
    assert_eq!(
        orig_str.strip_suffix(PATTERN9).map(OsStr::new),
        test_str.strip_suffix(PATTERN9)
    );

    println!("Testing OsStrExt::trim_start_matches()...");
    assert_eq!(
        orig_str.trim_start_matches(PATTERN0),
        test_str.trim_start_matches(PATTERN0)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN1),
        test_str.trim_start_matches(PATTERN1)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN2),
        test_str.trim_start_matches(PATTERN2)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN3),
        test_str.trim_start_matches(PATTERN3)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN4),
        test_str.trim_start_matches(PATTERN4)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN5),
        test_str.trim_start_matches(PATTERN5)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN6),
        test_str.trim_start_matches(PATTERN6)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN7),
        test_str.trim_start_matches(PATTERN7)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN8),
        test_str.trim_start_matches(PATTERN8)
    );
    assert_eq!(
        orig_str.trim_start_matches(PATTERN9),
        test_str.trim_start_matches(PATTERN9)
    );

    println!("Testing OsStrExt::trim_end_matches()...");
    assert_eq!(
        orig_str.trim_end_matches(PATTERN0),
        test_str.trim_end_matches(PATTERN0)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN1),
        test_str.trim_end_matches(PATTERN1)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN2),
        test_str.trim_end_matches(PATTERN2)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN3),
        test_str.trim_end_matches(PATTERN3)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN4),
        test_str.trim_end_matches(PATTERN4)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN5),
        test_str.trim_end_matches(PATTERN5)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN6),
        test_str.trim_end_matches(PATTERN6)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN7),
        test_str.trim_end_matches(PATTERN7)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN8),
        test_str.trim_end_matches(PATTERN8)
    );
    assert_eq!(
        orig_str.trim_end_matches(PATTERN9),
        test_str.trim_end_matches(PATTERN9)
    );

    println!("Testing OsStrExt::trim_matches()...");
    assert_eq!(test_str, test_str.trim_matches(PATTERN0));
    assert_eq!(test_str, test_str.trim_matches(PATTERN1));
    assert_eq!(test_str, test_str.trim_matches(PATTERN2));
    assert_eq!(
        orig_str.trim_matches(PATTERN4),
        test_str.trim_matches(PATTERN4)
    );
    assert_eq!(
        orig_str.trim_matches(PATTERN4),
        test_str.trim_matches(PATTERN4)
    );
    assert_eq!(
        orig_str.trim_matches(PATTERN5),
        test_str.trim_matches(PATTERN5)
    );
    assert_eq!(
        orig_str.trim_matches(PATTERN6),
        test_str.trim_matches(PATTERN6)
    );
    assert_eq!(test_str, test_str.trim_matches(PATTERN7));
    assert_eq!(
        orig_str.trim_matches(PATTERN8),
        test_str.trim_matches(PATTERN8)
    );
    assert_eq!(
        orig_str.trim_matches(PATTERN9),
        test_str.trim_matches(PATTERN9)
    );

    println!("Testing OsStrExt::trim_start()...");
    assert_eq!(orig_str.trim_start(), test_str.trim_start());

    println!("Testing OsStrExt::trim_end()...");
    assert_eq!(orig_str.trim_end(), test_str.trim_end());

    println!("Testing OsStrExt::trim()...");
    assert_eq!(orig_str.trim(), test_str.trim());

    println!("Testing OsStrExt::split()...");
    assert_eq!(
        orig_str.split(PATTERN0).collect::<Vec<_>>(),
        test_str.split(PATTERN0).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN1).collect::<Vec<_>>(),
        test_str.split(PATTERN1).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN2).collect::<Vec<_>>(),
        test_str.split(PATTERN2).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN3).collect::<Vec<_>>(),
        test_str.split(PATTERN3).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN4).collect::<Vec<_>>(),
        test_str.split(PATTERN4).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN5).collect::<Vec<_>>(),
        test_str.split(PATTERN5).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN6).collect::<Vec<_>>(),
        test_str.split(PATTERN6).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN7).collect::<Vec<_>>(),
        test_str.split(PATTERN7).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN8).collect::<Vec<_>>(),
        test_str.split(PATTERN8).collect::<Vec<_>>()
    );
    assert_eq!(
        orig_str.split(PATTERN9).collect::<Vec<_>>(),
        test_str.split(PATTERN9).collect::<Vec<_>>()
    );

    println!("Testing OsStrExt::split_inclusive()...");
    assert_eq!(
        orig_str.split_inclusive(PATTERN4).collect::<Vec<_>>(),
        test_str.split_inclusive(PATTERN4).collect::<Vec<_>>()
    );

    println!("Testing OsStrExt::split_whitespace()...");
    assert_eq!(
        orig_str.split_whitespace().collect::<Vec<_>>(),
        test_str.split_whitespace().collect::<Vec<_>>()
    );

    println!("Testing OsStrExt::split_at()...");
    for index in 0..orig_str.as_bytes().len() {
        if !orig_str.is_char_boundary(index) {
            continue;
        }

        let (a, b) = orig_str.split_at(index);
        let str_result = (OsStr::new(a), OsStr::new(b));
        let os_str_result = test_str.split_at(index);

        assert_eq!(str_result, os_str_result);
    }

    println!("Testing OsStrExt::lines()...");
    assert_eq!(
        orig_str.lines().collect::<Vec<_>>(),
        test_str.lines().collect::<Vec<_>>()
    );

    println!("Testing OsStrExt::replace()");
    assert_eq!(
        orig_str
            .replace("fox", "dog")
            .replace("dog", "cat")
            .as_str(),
        test_str
            .replace("fox", "dog")
            .replace("dog", "cat")
            .as_os_str()
    );

    println!("Testing OsStrExt::to_cstring()...");
    let c_str = CString::new(orig_str).expect("CString conversion failed");
    assert_eq!(orig_str.as_bytes(), c_str.as_bytes());
    assert_ne!(orig_str.as_bytes(), c_str.as_bytes_with_nul());
}
