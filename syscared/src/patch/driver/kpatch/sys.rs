// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{ffi::OsStr, fs, path::Path};

use log::debug;
use nix::errno::Errno;

use syscare_abi::PatchStatus;
use syscare_common::os::kernel;

const KPATCH_STATUS_DISABLE: &str = "0";
const KPATCH_STATUS_ENABLE: &str = "1";

pub fn load_patch<P: AsRef<Path>>(patch_file: P) -> std::io::Result<()> {
    let patch_file = patch_file.as_ref();

    debug!(
        "Kpatch: Inserting patch module '{}'...",
        patch_file.display()
    );
    kernel::insert_module(patch_file)
}

pub fn remove_patch<S: AsRef<OsStr>>(module_name: S) -> std::io::Result<()> {
    let module_name = module_name.as_ref();

    debug!(
        "Kpatch: Removing patch module '{}'...",
        module_name.to_string_lossy()
    );
    kernel::remove_module(module_name)
}

pub fn active_patch<P: AsRef<Path>>(status_file: P) -> std::io::Result<()> {
    let status_file = status_file.as_ref();

    debug!(
        "Kpatch: Writing '{}' to '{}'...",
        stringify!(KPATCH_STATUS_ENABLE),
        status_file.display()
    );
    fs::write(status_file, KPATCH_STATUS_ENABLE)
}

pub fn deactive_patch<P: AsRef<Path>>(status_file: P) -> std::io::Result<()> {
    let status_file = status_file.as_ref();

    debug!(
        "Kpatch: Writing '{}' to '{}'...",
        stringify!(KPATCH_STATUS_DISABLE),
        status_file.display()
    );
    fs::write(status_file, KPATCH_STATUS_DISABLE)
}

pub fn get_patch_status<P: AsRef<Path>>(status_file: P) -> std::io::Result<PatchStatus> {
    let status_file = status_file.as_ref();
    if !status_file.exists() {
        return Ok(PatchStatus::NotApplied);
    }

    debug!("Kpatch: Reading '{}'...", status_file.display());
    match fs::read_to_string(status_file)?.trim() {
        KPATCH_STATUS_DISABLE => Ok(PatchStatus::Deactived),
        KPATCH_STATUS_ENABLE => Ok(PatchStatus::Actived),
        _ => Err(std::io::Error::from(Errno::EINVAL)),
    }
}
