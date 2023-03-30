use std::ffi::{OsStr, OsString};

use std::os::unix::prelude::OsStrExt as UnixOsStrExt;

pub struct Split<'a, P> {
    data:     &'a OsStr,
    pattern:  P,
    finished: bool,
}

impl<'a, P> Iterator for Split<'a, P>
where
    P: AsRef<OsStr>
{
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.data.find(&self.pattern) {
            Some(position) => {
                let pat_bytes = self.pattern.as_ref().as_bytes();
                let str_bytes = self.data.as_bytes();
                let lhs_bytes = &str_bytes[..position];
                let rhs_bytes = &str_bytes[(position + pat_bytes.len())..];

                self.data = OsStr::from_bytes(rhs_bytes);
                return Some(OsStr::from_bytes(lhs_bytes));
            },
            None => {
                self.finished = true;
                return Some(self.data);
            }
        }
    }
}

pub trait OsStrExt
where
    Self: AsRef<OsStr>
{
    fn find<P: AsRef<OsStr>>(&self, pattern: P) -> Option<usize> {
        let str_bytes = self.as_ref().as_bytes();
        let pat_bytes = pattern.as_ref().as_bytes();

        str_bytes.windows(pat_bytes.len()).position(|s| s == pat_bytes)
    }

    fn contains<S: AsRef<OsStr>>(&self, pattern: S) -> bool {
        self.as_ref().find(pattern).is_some()
    }

    fn starts_with<S: AsRef<OsStr>>(&self, needle: S) -> bool {
        let str_bytes    = self.as_ref().as_bytes();
        let needle_bytes = needle.as_ref().as_bytes();

        str_bytes.starts_with(needle_bytes)
    }

    fn ends_with<S: AsRef<OsStr>>(&self, needle: S) -> bool {
        let str_bytes    = self.as_ref().as_bytes();
        let needle_bytes = needle.as_ref().as_bytes();

        str_bytes.ends_with(needle_bytes)
    }

    fn strip_prefix<S: AsRef<OsStr>>(&self, prefix: S) -> Option<&OsStr> {
        let str_bytes    = self.as_ref().as_bytes();
        let prefix_bytes = prefix.as_ref().as_bytes();

        str_bytes.strip_prefix(prefix_bytes).map(OsStr::from_bytes)
    }

    fn strip_suffix<S: AsRef<OsStr>>(&self, suffix: S) -> Option<&OsStr> {
        let str_bytes    = self.as_ref().as_bytes();
        let suffix_bytes = suffix.as_ref().as_bytes();

        str_bytes.strip_suffix(suffix_bytes).map(OsStr::from_bytes)
    }

    fn split<'a, P: AsRef<OsStr>>(&'a self, pattern: P) -> Split<'a, P> {
        Split {
            data: self.as_ref(),
            pattern,
            finished: false,
        }
    }

    fn split_at(&self, index: usize) -> (&OsStr, &OsStr) {
        let str_bytes = self.as_ref().as_bytes();
        let lhs_bytes = &str_bytes[..index];
        let rhs_bytes = &str_bytes[index..];

        (OsStr::from_bytes(lhs_bytes), OsStr::from_bytes(rhs_bytes))
    }

    fn trim_start(&self) -> &OsStr {
        let str_bytes = self.as_ref().as_bytes();

        let mut start = 0;
        for idx in 0..str_bytes.len() {
            if !str_bytes[idx].is_ascii_whitespace() {
                break;
            }
            start += 1;
        }

        OsStr::from_bytes(&str_bytes[start..])
    }

    fn trim_end(&self) -> &OsStr {
        let str_bytes = self.as_ref().as_bytes();

        let mut end = str_bytes.len();
        for idx in (0..str_bytes.len()).rev() {
            if !str_bytes[idx].is_ascii_whitespace() {
                break;
            }
            end -= 1;
        }

        OsStr::from_bytes(&str_bytes[..end])
    }

    fn trim(&self) -> &OsStr {
        self.trim_start().trim_end()
    }
}

impl OsStrExt for OsStr {}

pub trait OsStringExt {
    fn concat<S: AsRef<OsStr>>(self, s: S) -> Self;
}

impl OsStringExt for OsString {
    fn concat<S: AsRef<OsStr>>(mut self, s: S) -> Self {
        self.push(s);
        self
    }
}
