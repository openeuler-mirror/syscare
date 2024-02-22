// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::ffi::{CStr, CString};
use std::ffi::{OsStr, OsString};
use std::path::Path;

use std::os::unix::prelude::OsStrExt as UnixOsStrExt;

use log::{error, trace};
use nix::libc::{c_void, getxattr, setxattr, PATH_MAX};

use crate::util::fs;
use crate::util::os_str::OsStrExt;

const SELINUX_ENFORCE_FILE: &str = "/sys/fs/selinux/enforce";
const SELINUX_SECURITY_CONTEXT: &str = "security.selinux";
const SELINUX_SECURITY_CONTEXT_SPLITTER: char = ':';
const SELINUX_SECURITY_CONTEXT_TYPE_NUM: usize = 4;

#[derive(Debug, PartialEq, Eq)]
pub enum SELinuxStatus {
    Permissive,
    Enforcing,
    Disabled,
}

impl From<u32> for SELinuxStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => SELinuxStatus::Permissive,
            1 => SELinuxStatus::Enforcing,
            _ => SELinuxStatus::Disabled,
        }
    }
}

impl std::fmt::Display for SELinuxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

enum SecurityContextType {
    User = 0,
    Role = 1,
    Type = 2,
    Range = 3,
}

pub fn set_enforce(value: SELinuxStatus) -> std::io::Result<()> {
    if (value != SELinuxStatus::Permissive) || (value != SELinuxStatus::Enforcing) {
        error!("value \"{}\" is invalid", value);
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Set enforce failed",
        ));
    }
    fs::write(SELINUX_ENFORCE_FILE, value.to_string()).map_err(|e| {
        error!("{}", e);
        std::io::Error::new(e.kind(), "Set enforce failed")
    })
}

pub fn get_enforce() -> std::io::Result<SELinuxStatus> {
    match fs::read_to_string(SELINUX_ENFORCE_FILE) {
        Ok(status) => Ok(SELinuxStatus::from(
            status.parse::<u32>().unwrap_or_default(),
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SELinuxStatus::Disabled),
        Err(e) => {
            error!("{}", e);
            Err(std::io::Error::new(e.kind(), "Get enforce failed"))
        }
    }
}

pub fn read_security_context<P>(path: P) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    let mut buf = [0u8; PATH_MAX as usize];

    let sec_cxt_path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
    let sec_cxt_name = CString::new(SELINUX_SECURITY_CONTEXT).unwrap();
    let sec_cxt_value = buf.as_mut_ptr();
    let sec_cxt_size = PATH_MAX as usize;

    let result = unsafe {
        let len = getxattr(
            sec_cxt_path.as_ptr(),
            sec_cxt_name.as_ptr(),
            sec_cxt_value as *mut c_void,
            sec_cxt_size,
        );
        if len <= 0 {
            let e = std::io::Error::last_os_error();
            return Err(std::io::Error::new(
                e.kind(),
                format!(
                    "Cannot read security context from \"{}\", {}",
                    path.as_ref().display(),
                    e.to_string().to_lowercase()
                ),
            ));
        }
        OsStr::from_bytes(
            CStr::from_bytes_with_nul(&buf[0..len as usize])
                .unwrap()
                .to_bytes(),
        )
        .to_os_string()
    };

    trace!("read security context: {:?}", result);
    Ok(result)
}

pub fn write_security_context<P, S>(path: P, scontext: S) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let sec_cxt_path = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
    let sec_cxt_name = CString::new(SELINUX_SECURITY_CONTEXT).unwrap();
    let sec_cxt_value = CString::new(scontext.as_ref().as_bytes()).unwrap();
    let sec_cxt_size = scontext.as_ref().len();
    trace!("write security context: {:?}", sec_cxt_value);

    unsafe {
        let ret = setxattr(
            sec_cxt_path.as_ptr(),
            sec_cxt_name.as_ptr(),
            sec_cxt_value.as_ptr() as *const c_void,
            sec_cxt_size,
            0,
        );
        if ret != 0 {
            let e = std::io::Error::last_os_error();
            return Err(std::io::Error::new(
                e.kind(),
                format!(
                    "Cannot write security context to {{{}}}, {}",
                    path.as_ref().display(),
                    e.to_string().to_lowercase()
                ),
            ));
        }
    }

    Ok(())
}

#[inline(always)]
fn get_security_context<P>(file_path: P, sc_type: SecurityContextType) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    let scontext = read_security_context(file_path.as_ref())?;
    let sgroup = scontext
        .split(SELINUX_SECURITY_CONTEXT_SPLITTER)
        .collect::<Vec<_>>();
    assert_eq!(sgroup.len(), SELINUX_SECURITY_CONTEXT_TYPE_NUM);

    Ok(sgroup[sc_type as usize].to_owned())
}

#[inline(always)]
fn set_security_context<P, S>(
    file_path: P,
    sc_type: SecurityContextType,
    value: S,
) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let old_scontext = read_security_context(&file_path)?;

    let mut sgroup = old_scontext
        .split(SELINUX_SECURITY_CONTEXT_SPLITTER)
        .collect::<Vec<_>>();
    sgroup[sc_type as usize] = value.as_ref();

    let mut new_scontext_buf = sgroup
        .into_iter()
        .flat_map(|s| {
            let mut buf = s.as_bytes().to_vec();
            buf.push(b':');
            buf
        })
        .collect::<Vec<_>>();
    new_scontext_buf.pop();

    let new_scontext = OsStr::from_bytes(&new_scontext_buf);
    if old_scontext != new_scontext {
        write_security_context(&file_path, new_scontext)?;
    }

    Ok(())
}

pub fn get_security_context_user<P>(file_path: P) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    get_security_context(file_path, SecurityContextType::User)
}

pub fn get_security_context_role<P>(file_path: P) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    get_security_context(file_path, SecurityContextType::Role)
}

pub fn get_security_context_type<P>(file_path: P) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    get_security_context(file_path, SecurityContextType::Type)
}

pub fn get_security_context_range<P>(file_path: P) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
{
    get_security_context(file_path, SecurityContextType::Range)
}

pub fn set_security_context_user<P, S>(file_path: P, value: S) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    set_security_context(file_path, SecurityContextType::User, value)
}

pub fn set_security_context_role<P, S>(file_path: P, value: S) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    set_security_context(file_path, SecurityContextType::Role, value)
}

pub fn set_security_context_type<P, S>(file_path: P, value: S) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    set_security_context(file_path, SecurityContextType::Type, value)
}

pub fn set_security_context_range<P, S>(file_path: P, value: S) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    set_security_context(file_path, SecurityContextType::Range, value)
}
