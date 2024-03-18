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

use std::{ffi::OsStr, path::Path};

use anyhow::Result;

const KEXEC_PATH: &str = "kexec";
const SYSTEMCTL_PATH: &str = "systemctl";

use super::platform;
use crate::{concat_os, process::Command};

pub fn version() -> &'static OsStr {
    platform::release()
}

pub fn load<P, Q>(kernel: P, initramfs: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    Command::new(KEXEC_PATH)
        .arg("--load")
        .arg(kernel.as_ref())
        .arg(concat_os!("--initrd=", initramfs.as_ref()))
        .arg("--reuse-cmdline")
        .run_with_output()?
        .exit_ok()
}

pub fn unload() -> Result<()> {
    Command::new(KEXEC_PATH)
        .arg("--unload")
        .run_with_output()?
        .exit_ok()
}

pub fn systemd_exec() -> Result<()> {
    Command::new(SYSTEMCTL_PATH)
        .arg("kexec")
        .run_with_output()?
        .exit_ok()
}

pub fn force_exec() -> Result<()> {
    Command::new(KEXEC_PATH)
        .arg("--exec")
        .run_with_output()?
        .exit_ok()
}
