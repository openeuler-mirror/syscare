use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use nix::unistd::getpid;

use crate::util::fs;

pub fn id() -> i32 {
    lazy_static! {
        static ref PROCESS_ID: i32 = getpid().as_raw();
    }
    *PROCESS_ID
}

pub fn path() -> &'static Path {
    lazy_static! {
        static ref PROCESS_PATH: PathBuf =
            std::env::current_exe().expect("Read process path failed");
    }
    PROCESS_PATH.as_path()
}

pub fn name() -> &'static OsStr {
    lazy_static! {
        static ref PROCESS_NAME: OsString = fs::file_name(path());
    }
    PROCESS_NAME.as_os_str()
}
