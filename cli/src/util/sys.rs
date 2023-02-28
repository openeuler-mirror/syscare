use std::ffi::{OsString, OsStr, CStr};
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use lazy_static::*;

use super::fs;

pub fn cpu_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn cpu_num() -> usize {
    lazy_static! {
        static ref CPU_NUM: usize = init_cpu_num();
    }

    fn init_cpu_num() -> usize {
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
        max_cpu_id + 1
    }

    *CPU_NUM
}

pub fn user_id() -> u32 {
    lazy_static! {
        static ref USER_ID: u32 = unsafe { libc::getuid() };
    }
    *USER_ID
}

pub fn process_id() -> i32 {
    lazy_static! {
        static ref PROCESS_ID: i32 = unsafe { libc::getpid() };
    }
    *PROCESS_ID
}

pub fn process_path() -> &'static Path {
    lazy_static! {
        static ref PROCESS_PATH: PathBuf = std::env::current_exe()
            .expect("Read process path failed");
    }
    PROCESS_PATH.as_path()
}

pub fn process_name() -> &'static OsStr {
    lazy_static! {
        static ref PROCESS_NAME: OsString = fs::file_name(
            std::env::current_exe().expect("Read process path failed")
        );
    }
    PROCESS_NAME.as_os_str()
}

pub fn kernel_version() -> &'static OsStr {
    lazy_static! {
        static ref KERNEL_VERSION: OsString = unsafe { init_kernel_version() };
    }

    unsafe fn init_kernel_version() -> OsString {
        let mut buf = libc::utsname {
            sysname:    [0; 65],
            nodename:   [0; 65],
            release:    [0; 65],
            version:    [0; 65],
            machine:    [0; 65],
            domainname: [0; 65]
        };
        let ret = libc::uname(&mut buf);
        assert_eq!(ret, 0);

        OsStr::from_bytes(
            CStr::from_ptr(buf.release.as_ptr()).to_bytes()
        ).to_os_string()
    }

    KERNEL_VERSION.as_os_str()
}
