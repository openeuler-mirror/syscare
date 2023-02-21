use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::os::unix::prelude::OsStrExt;

/* Contains */
pub trait OsStrContains
where
    Self: AsRef<OsStr>
{
    fn contains<S: AsRef<OsStr>>(&self, needle: S) -> bool {
        let needle = needle.as_ref();

        std::os::unix::prelude::OsStrExt::as_bytes(self.as_ref())
            .windows(needle.len())
            .position(|window| window == needle.as_bytes())
            .is_some()
    }
}

impl OsStrContains for OsStr {}
impl OsStrContains for Path {}

/* Split */
pub struct Split<'a> {
    data:     &'a [u8],
    spliter:  char,
    finished: bool,
}

impl<'a> Iterator for Split<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.data.iter().position(|b| b == &(self.spliter as u8)) {
            None => {
                if self.finished {
                    None
                } else {
                    self.finished = true;
                    Some(OsStr::from_bytes(self.data))
                }
            },
            Some(idx) => {
                let str = Some(OsStr::from_bytes(&self.data[..idx]));
                self.data = &self.data[idx + 1..];
                str
            }
        }
    }
}

pub trait OsStrSplit
where
    Self: AsRef<OsStr>
{
    fn split(&self, spliter: char) -> Split {
        Split {
            data: self.as_ref().as_bytes(),
            spliter,
            finished: false,
        }
    }
}

impl OsStrSplit for OsStr {}
impl OsStrSplit for Path {}

/* Concat */
pub trait OsStrConcat {
    fn concat<T: AsRef<OsStr>>(self, s: T) -> Self;
}

impl OsStrConcat for OsString {
    fn concat<T: AsRef<OsStr>>(mut self, s: T) -> Self {
        self.push(s);
        self
    }
}
