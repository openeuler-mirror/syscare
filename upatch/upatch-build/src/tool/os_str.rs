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
