use std::ffi::CStr;
use std::ffi::{OsStr, OsString};
use std::os::unix::prelude::OsStringExt;
use std::path::{PathBuf, Path};

use lazy_static::*;

struct UserInfo {
    name:   OsString,
    passwd: OsString,
    uid:    u32,
    gid:    u32,
    gecos:  OsString,
    home:   PathBuf,
    shell:  PathBuf,
}

#[inline(always)]
fn info() -> &'static UserInfo {
    lazy_static! {
        static ref USER_INFO: UserInfo = unsafe {
            let uid = libc::getuid();
            let mut pwd = std::mem::MaybeUninit::zeroed().assume_init();
            let mut buf = Vec::with_capacity(match libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) {
                n if n < 0 => 16384 as usize,
                n => n as usize,
            });
            let buflen = buf.capacity();
            let mut result = std::ptr::null_mut();

            let ret = libc::getpwuid_r(uid, &mut pwd, buf.as_mut_ptr(), buflen, &mut result);
            assert_eq!(ret, 0);
            assert!(!result.is_null());

            UserInfo {
                name:   OsString::from_vec(CStr::from_ptr(pwd.pw_name).to_bytes().to_vec()),
                passwd: OsString::from_vec(CStr::from_ptr(pwd.pw_passwd).to_bytes().to_vec()),
                uid:    pwd.pw_uid,
                gid:    pwd.pw_gid,
                gecos:  OsString::from_vec(CStr::from_ptr(pwd.pw_gecos).to_bytes().to_vec()),
                home:   PathBuf::from(OsString::from_vec(CStr::from_ptr(pwd.pw_dir).to_bytes().to_vec())),
                shell:  PathBuf::from(OsString::from_vec(CStr::from_ptr(pwd.pw_shell).to_bytes().to_vec())),
            }
        };
    }
    &USER_INFO
}

pub fn name() -> &'static OsStr {
    &info().name
}

pub fn passwd() -> &'static OsStr {
    &info().passwd
}

pub fn id() -> u32 {
    info().uid
}

pub fn gid() -> u32 {
    info().gid
}

pub fn gecos() -> &'static OsStr {
    &info().gecos
}

pub fn home() -> &'static Path {
    &info().home
}

pub fn shell() -> &'static Path {
    &info().shell
}
