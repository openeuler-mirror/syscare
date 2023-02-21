use std::ffi::{OsString, OsStr};
use std::path::{Path, PathBuf};

use lazy_static::*;

use super::fs;

lazy_static! {
    static ref CPU_NUM:      String   = SysInitializer::init_cpu_num();
    static ref PROCESS_PATH: PathBuf  = SysInitializer::init_process_path();
    static ref PROCESS_NAME: OsString = SysInitializer::init_process_name();
}

struct SysInitializer;

impl SysInitializer {
    pub fn init_cpu_num() -> String {
        let cpu_online_info = fs::read_to_string("/sys/devices/system/cpu/online")
            .expect("Read cpu number failed");

        let max_cpu_id = cpu_online_info
            .trim()
            .split('-')
            .last()
            .map(str::parse::<usize>)
            .and_then(Result::ok)
            .unwrap_or_default();

        // cpu id starts from 0
        (max_cpu_id + 1).to_string()
    }

    pub fn init_process_path() -> PathBuf {
        std::env::current_exe().expect("Read process path failed")
    }

    pub fn init_process_name() -> OsString {
        fs::file_name(std::env::current_exe().expect("Read process path failed"))
    }
}

pub const fn cpu_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn cpu_num() -> &'static str {
    CPU_NUM.as_str()
}

pub fn process_id() -> u32 {
    std::process::id()
}

pub fn process_path() -> &'static Path {
    PROCESS_PATH.as_path()
}

pub fn process_name() -> &'static OsStr {
    PROCESS_NAME.as_os_str()
}
