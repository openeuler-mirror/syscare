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

use std::ffi::{OsStr, OsString};
use std::path::Path;

use anyhow::Result;
use lazy_static::lazy_static;

lazy_static! {
    static ref KEXEC: ExternCommand = ExternCommand::new("kexec");
    static ref SYSTEMCTL: ExternCommand = ExternCommand::new("systemcl");
}

use super::platform;
use crate::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    os_str::OsStringExt,
};

pub fn version() -> &'static OsStr {
    platform::release()
}

pub fn load<P, Q>(kernel: P, initramfs: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let exit_status = KEXEC.execvp(
        ExternCommandArgs::new()
            .arg("--load")
            .arg(kernel.as_ref())
            .arg(OsString::from("--initrd=").concat(initramfs.as_ref()))
            .arg("--reuse-cmdline"),
    )?;
    exit_status.check_exit_code()
}

pub fn systemd_exec() -> Result<()> {
    SYSTEMCTL
        .execvp(ExternCommandArgs::new().arg("kexec"))?
        .check_exit_code()
}

pub fn direct_exec() -> Result<()> {
    KEXEC
        .execvp(ExternCommandArgs::new().arg("--exec"))?
        .check_exit_code()
}
