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

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt as UnixOsStringExt;
use std::path::Path;

use anyhow::{bail, ensure, Context, Result};

use crate::{concat_os, ffi::OsStrExt, fs};

const SELINUX_SYS_FILE: &str = "/sys/fs/selinux/enforce";
const SELINUX_XATTR_NAME: &str = "security.selinux";
const SELINUX_XATTR_SPLITTER: &str = ":";
const SECURITY_XATTR_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Permissive,
    Enforcing,
    Disabled,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

pub fn get_status() -> Result<Status> {
    if !Path::new(SELINUX_SYS_FILE).is_file() {
        return Ok(Status::Disabled);
    }

    let value = OsString::from_vec(fs::read(SELINUX_SYS_FILE)?)
        .to_string_lossy()
        .parse::<u32>()
        .context("Failed to parse selinux status")?;

    Ok(match value {
        0 => Status::Permissive,
        1 => Status::Enforcing,
        _ => Status::Disabled,
    })
}

pub fn set_status(value: Status) -> Result<()> {
    if (value != Status::Permissive) && (value != Status::Enforcing) {
        bail!("Status {} is invalid", value);
    }
    fs::write(SELINUX_SYS_FILE, value.to_string())?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContext {
    pub user: OsString,
    pub role: OsString,
    pub kind: OsString,
    pub level: OsString,
}

impl AsRef<SecurityContext> for SecurityContext {
    fn as_ref(&self) -> &SecurityContext {
        self
    }
}

pub fn get_security_context<P>(file_path: P) -> Result<SecurityContext>
where
    P: AsRef<Path>,
{
    let value = fs::getxattr(file_path, SELINUX_XATTR_NAME)?;
    let data = value.split(SELINUX_XATTR_SPLITTER).collect::<Vec<_>>();
    ensure!(
        data.len() == SECURITY_XATTR_LEN,
        "Failed to parse selinux security context"
    );

    Ok(SecurityContext {
        user: data[0].to_os_string(),
        role: data[1].to_os_string(),
        kind: data[2].to_os_string(),
        level: data[3].to_os_string(),
    })
}

pub fn set_security_context<P, S>(file_path: P, value: S) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<SecurityContext>,
{
    let old_value = get_security_context(&file_path)?;
    let new_value = value.as_ref();

    if &old_value == new_value {
        return Ok(());
    }

    let new_context = concat_os!(
        &new_value.user,
        SELINUX_XATTR_SPLITTER,
        &new_value.role,
        SELINUX_XATTR_SPLITTER,
        &new_value.kind,
        SELINUX_XATTR_SPLITTER,
        &new_value.level,
    );
    fs::setxattr(&file_path, SELINUX_XATTR_NAME, new_context)?;

    Ok(())
}
