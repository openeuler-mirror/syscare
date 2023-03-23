use std::ffi::{OsString, OsStr};
use std::os::unix::prelude::OsStrExt;

use std::path::PathBuf;
use std::io::BufReader;

use crate::util::fs;
use crate::util::raw_line::RawLines;

struct MountInfoParser<'a> {
    data: &'a [u8],
    pos: usize,
    idx: usize,
}

impl<'a> MountInfoParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            idx: 0,
        }
    }
}

impl<'a> Iterator for MountInfoParser<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<Self::Item> {
        const NORMAL_PATTERN: &str = " ";
        const OPTION_PATTERN: &str = " - ";

        let start = self.pos;
        let end   = self.data.len();
        if start == end {
            return None;
        }
        /*
         * As the description in man pages of mountinfo writes:
         * (7) optional fields: zero or more fields of the form "tag[:value]"; see below.
         * (8) separator: the end of the optional fields is marked by a single hyphen.
         *
         * Normally, we use " " to match the string
         * But for optional fields, we use " - "
         */
        let pat = match self.idx != 6 {
            true  => NORMAL_PATTERN,
            false => OPTION_PATTERN,
        }.as_bytes();
        /*
         * Try to match specific pattern
         * if success, the rest data is what we want
         * if failed, return remaining chars of last field
         */
        let pat_len = pat.len();
        let remains = &self.data[start..end];
        match remains.windows(pat_len).position(|bytes| bytes == pat) {
            Some(pos) => {
                let str = OsStr::from_bytes(&self.data[start..(start + pos)]);
                self.pos += pos + pat_len;
                self.idx += 1;

                Some(str)
            }
            None => {
                let str = OsStr::from_bytes(&self.data[start..end]);
                self.pos = end;
                self.idx += 1;

                Some(str)
            }
        }
    }
}

#[derive(Debug)]
pub struct MountInfo {
    pub mount_id:    u32,
    pub parent_id:   u32,
    pub device_id:   OsString,
    pub root:        PathBuf,
    pub mount_point: PathBuf,
    pub mount_opts:  OsString,
    pub optional:    OsString,
    pub filesystem:  OsString,
    pub source:      PathBuf,
    pub super_opts:  OsString,
}

pub fn enumerate() -> std::io::Result<Vec<MountInfo>> {
    const MOUNTINFO_FILE: &str = "/proc/self/mountinfo";

    let mut result = Vec::new();

    for read_line in RawLines::from(BufReader::new(fs::open_file(MOUNTINFO_FILE)?)) {
        let line = read_line?;
        let info = MountInfoParser::new(line.as_bytes()).collect::<Vec<_>>();
        assert_eq!(info.len(), 10);

        result.push(MountInfo {
            mount_id:    info[0].to_string_lossy().parse::<u32>().unwrap_or_default(),
            parent_id:   info[1].to_string_lossy().parse::<u32>().unwrap_or_default(),
            device_id:   info[2].to_os_string(),
            root:        PathBuf::from(info[3]),
            mount_point: PathBuf::from(info[4]),
            mount_opts:  info[5].to_os_string(),
            optional:    info[6].to_os_string(),
            filesystem:  info[7].to_os_string(),
            source:      PathBuf::from(info[8]),
            super_opts:  info[9].to_os_string(),
        })
    }

    Ok(result)
}
