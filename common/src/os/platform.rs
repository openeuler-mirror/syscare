use std::ffi::CStr;
use std::ffi::{OsStr, OsString};
use std::os::unix::prelude::OsStringExt;

use lazy_static::lazy_static;

struct PlatformInfo {
    sysname:  OsString,
    hostname: OsString,
    release:  OsString,
    version:  OsString,
    arch:     OsString,
    domain:   OsString,
}

#[inline(always)]
fn info() -> &'static PlatformInfo {
    lazy_static! {
        static ref PLATFORM_INFO: PlatformInfo = unsafe {
            let mut buf = std::mem::zeroed::<libc::utsname>();

            let ret = libc::uname(&mut buf);
            assert_eq!(ret, 0);

            PlatformInfo {
                sysname:  OsString::from_vec(CStr::from_ptr(buf.sysname.as_ptr()).to_bytes().to_vec()),
                hostname: OsString::from_vec(CStr::from_ptr(buf.nodename.as_ptr()).to_bytes().to_vec()),
                release:  OsString::from_vec(CStr::from_ptr(buf.release.as_ptr()).to_bytes().to_vec()),
                version:  OsString::from_vec(CStr::from_ptr(buf.version.as_ptr()).to_bytes().to_vec()),
                arch:     OsString::from_vec(CStr::from_ptr(buf.machine.as_ptr()).to_bytes().to_vec()),
                domain:   OsString::from_vec(CStr::from_ptr(buf.domainname.as_ptr()).to_bytes().to_vec()),
            }
        };
    }
    &PLATFORM_INFO
}

pub fn sysname() -> &'static OsStr {
    &info().sysname
}

pub fn hostname() -> &'static OsStr {
    &info().hostname
}

pub fn release() -> &'static OsStr {
    &info().release
}

pub fn version() -> &'static OsStr {
    &info().version
}

pub fn arch() -> &'static OsStr {
    &info().arch
}

pub fn domain() -> &'static OsStr {
    &info().domain
}
