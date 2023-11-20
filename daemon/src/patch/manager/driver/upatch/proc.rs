use std::{
    convert::TryFrom,
    ffi::{OsStr, OsString},
    fs::File,
};

use anyhow::Result;

use syscare_common::util::{fs, os_str::OsStrExt, raw_line::RawLines};

#[derive(Debug)]
pub struct ProcMapping {
    pub address: OsString,
    pub permission: OsString,
    pub offset: OsString,
    pub dev: OsString,
    pub inode: OsString,
    pub path_name: OsString,
}

impl TryFrom<OsString> for ProcMapping {
    type Error = anyhow::Error;

    fn try_from(value: OsString) -> std::result::Result<Self, Self::Error> {
        let values = value.split_whitespace().collect::<Vec<_>>();
        let parse_value = |value: Option<&&OsStr>| -> OsString {
            value.map(|s| s.to_os_string()).unwrap_or_default()
        };

        Ok(Self {
            address: parse_value(values.get(0)),
            permission: parse_value(values.get(1)),
            offset: parse_value(values.get(2)),
            dev: parse_value(values.get(3)),
            inode: parse_value(values.get(4)),
            path_name: parse_value(values.get(5)),
        })
    }
}

pub struct ProcMappingReader {
    lines: RawLines<File>,
}

impl ProcMappingReader {
    pub fn new(pid: i32) -> Result<Self> {
        let file_path = format!("/proc/{}/maps", pid);
        let lines = RawLines::from(fs::open_file(file_path)?);

        Ok(Self { lines })
    }
}

impl Iterator for ProcMappingReader {
    type Item = ProcMapping;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines
            .next()
            .and_then(Result::ok)
            .map(ProcMapping::try_from)
            .and_then(Result::ok)
    }
}

#[test]
fn test() {
    for mapping in ProcMappingReader::new(1).unwrap() {
        println!("{:#?}", mapping);
    }
}
