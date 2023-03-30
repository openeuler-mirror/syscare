use std::ffi::OsString;
use std::os::unix::prelude::OsStringExt;
use std::path::PathBuf;
use std::{path::Path, fs::File};
use std::io::BufReader;
use std::collections::{HashSet, HashMap};
use std::os::unix::ffi::OsStrExt;

use log::debug;

use crate::cmd::RawLines;

use super::read_build_id;

const SEPARATOR_NUM: usize = 4;
const SEPARATOR: [u8; SEPARATOR_NUM] = [58, 58, 58, 58]; // OsString = ::::
pub const LINK_LOG: &str = "upatch-link.log";

#[derive(Clone)]
pub struct LinkMessage {
    pub build_id: Option<Vec<u8>>,
    pub objects: HashSet<PathBuf>,
}

impl LinkMessage {
    pub fn new() -> Self {
        Self {
            build_id: None,
            objects: HashSet::new(),
        }
    }

    pub fn is_empty(&self) -> bool{
        self.build_id.is_none() && self.objects.is_empty()
    }
}

impl std::fmt::Debug for LinkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("\nbuild_id: {:?}, \nobjects: {:?}\n", self.build_id, self.objects))
    }
}

pub struct LinkMessages {
    link_message: HashMap<OsString, LinkMessage>
}

impl LinkMessages {
    pub fn new() -> Self {
        Self {
            link_message: HashMap::new()
        }
    }

    pub fn from<P: AsRef<Path>>(path: P, read_build_id_flag: bool) -> std::io::Result<Self> {
        let file = std::fs::File::open(path.as_ref().join(LINK_LOG))?;
        Ok(Self {
            link_message: Self::parse(file, path, read_build_id_flag)?
        })
    }

    fn parse<P: AsRef<Path>>(file: File, path: P, read_build_id_flag: bool) -> std::io::Result<HashMap<OsString, LinkMessage>> {
        let mut result: HashMap<OsString, LinkMessage> = HashMap::new();
        for line in RawLines::from(BufReader::new(file)) {
            let data = line?;
            if !data.is_empty() {
                let (binary, object) = Self::parse_line(&data)?;
                if !object.starts_with(path.as_ref()) {
                    continue;
                }
                match result.contains_key(&binary) {
                    true => { result.get_mut(&binary).expect("get binary error").objects.insert(object); },
                    false => {
                        let mut message = LinkMessage::new();
                        message.build_id = match read_build_id_flag {
                            true => match read_build_id(&binary) {
                                Ok(Some(build_id)) => Some(build_id),
                                _ => {
                                    debug!("parse link log: read {:?} failed!", &binary);
                                    None
                                },
                            },
                            false => None,
                        };
                        message.objects.insert(object);
                        result.insert(binary, message);
                    },
                };
            }
        }
        Ok(result)
    }

    fn parse_line(line: &OsString) -> std::io::Result<(OsString, PathBuf)> {
        let line_bytes = line.as_bytes();
        for i in 0..(line_bytes.len() - SEPARATOR_NUM) {
            if SEPARATOR.eq(&line_bytes[i..(i + SEPARATOR_NUM)]) {
                return Ok((OsString::from_vec(line_bytes[0..i].to_vec()), PathBuf::from(OsString::from_vec(line_bytes[(i + SEPARATOR_NUM)..].to_vec()))));
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("parse log file failed!")))
    }
}

impl LinkMessages {
    pub fn get_objects_from_build_id(&self, build_id: &Vec<u8>) -> Option<(&OsString, &LinkMessage)> {
        for (binary, link_message) in &self.link_message {
            match &link_message.build_id {
                Some(id)  => if build_id.eq(id) {
                    return Some((binary, &link_message));
                },
                None => (),
            }
        }
        None
    }

    pub fn get_objects_from_binary(&self, binary_path: &OsString) -> Option<&LinkMessage> {
        self.link_message.get(binary_path)
    }
}

impl std::fmt::Debug for LinkMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.link_message))
    }
}