use std::ffi::CStr;
use std::ffi::{OsString, OsStr};
use std::os::unix::prelude::OsStrExt;

pub fn get_uid() -> u32 {
    unsafe { libc::getuid() as u32 }
}

pub fn get_arch() -> &'static str {
    std::env::consts::ARCH
}

pub fn get_kernel_version() -> std::io::Result<OsString> {
    let kernel_version = unsafe {
        let mut buf = std::mem::zeroed();
        if libc::uname(&mut buf) != 0 {
            return Err(std::io::Error::last_os_error());
        }

        OsStr::from_bytes(
            CStr::from_ptr(buf.release.as_ptr()).to_bytes()
        ).to_os_string()
    };

    Ok(kernel_version)
}
