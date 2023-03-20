use std::ffi::{OsStr, OsString};

use std::os::unix::prelude::OsStrExt as UnixOsStrExt;

pub trait OsStrExt
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

    fn starts_with<S: AsRef<OsStr>>(&self, needle: S) -> bool {
        let os_str_bytes = self.as_ref().as_bytes();
        let needle_bytes = needle.as_ref().as_bytes();

        os_str_bytes.starts_with(needle_bytes)
    }

    fn ends_with<S: AsRef<OsStr>>(&self, needle: S) -> bool {
        let os_str_bytes = self.as_ref().as_bytes();
        let needle_bytes = needle.as_ref().as_bytes();

        os_str_bytes.ends_with(needle_bytes)
    }

    fn strip_prefix<S: AsRef<OsStr>>(&self, prefix: S) -> Option<&OsStr> {
        let os_str_bytes = self.as_ref().as_bytes();
        let prefix_bytes = prefix.as_ref().as_bytes();

        os_str_bytes.strip_prefix(prefix_bytes).map(OsStr::from_bytes)
    }

    fn strip_suffix<S: AsRef<OsStr>>(&self, suffix: S) -> Option<&OsStr> {
        let os_str_bytes = self.as_ref().as_bytes();
        let suffix_bytes = suffix.as_ref().as_bytes();

        os_str_bytes.strip_suffix(suffix_bytes).map(OsStr::from_bytes)
    }

    fn split<'a>(&'a self, spliter: char) -> Box<dyn Iterator<Item = &OsStr> + 'a> {
        let iter = self.as_ref()
            .as_bytes()
            .split(move|byte| byte == &(spliter as u8))
            .map(|slice|OsStr::from_bytes(slice));

        Box::new(iter)
    }

    fn trim_start(&self) -> &OsStr {
        let bytes = self.as_ref().as_bytes();

        let mut start = 0;
        for idx in 0..bytes.len() {
            if !bytes[idx].is_ascii_whitespace() {
                break;
            }
            start += 1;
        }

        OsStr::from_bytes(&bytes[start..])
    }

    fn trim_end(&self) -> &OsStr {
        let bytes = self.as_ref().as_bytes();

        let mut end = bytes.len();
        for idx in (0..bytes.len()).rev() {
            if !bytes[idx].is_ascii_whitespace() {
                break;
            }
            end -= 1;
        }

        OsStr::from_bytes(&bytes[..end])
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
