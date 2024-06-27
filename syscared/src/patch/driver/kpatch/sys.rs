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

use std::ffi::{CString, OsString};

use anyhow::{anyhow, bail, Context, Result};
use log::debug;
use nix::kmod;

use syscare_abi::PatchStatus;
use syscare_common::{ffi::OsStrExt, fs, os};

use crate::patch::entity::KernelPatch;

const SYS_MODULE_DIR: &str = "/sys/module";
const KPATCH_STATUS_DISABLED: &str = "0";
const KPATCH_STATUS_ENABLED: &str = "1";

pub fn list_kernel_modules() -> Result<Vec<OsString>> {
    let module_names = fs::list_dirs(SYS_MODULE_DIR, fs::TraverseOptions { recursive: false })?
        .into_iter()
        .filter_map(|dir| dir.file_name().map(|name| name.to_os_string()))
        .collect();

    Ok(module_names)
}

pub fn selinux_relable_patch(patch: &KernelPatch) -> Result<()> {
    const KPATCH_PATCH_SEC_TYPE: &str = "modules_object_t";

    if os::selinux::get_status()? != os::selinux::Status::Enforcing {
        return Ok(());
    }

    debug!(
        "Relabeling patch module '{}'...",
        patch.module_name.to_string_lossy()
    );
    let mut sec_context = os::selinux::get_security_context(&patch.patch_file)?;
    if sec_context.kind != KPATCH_PATCH_SEC_TYPE {
        sec_context.kind = OsString::from(KPATCH_PATCH_SEC_TYPE);
        os::selinux::set_security_context(&patch.patch_file, sec_context)?;
    }

    Ok(())
}

pub fn read_patch_status(patch: &KernelPatch) -> Result<PatchStatus> {
    let sys_file = patch.sys_file.as_path();
    debug!("Reading {}", sys_file.display());

    let status = match fs::read_to_string(sys_file) {
        Ok(str) => match str.trim() {
            KPATCH_STATUS_DISABLED => Ok(PatchStatus::Deactived),
            KPATCH_STATUS_ENABLED => Ok(PatchStatus::Actived),
            _ => bail!("Kpatch: Invalid patch status"),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PatchStatus::NotApplied),
        Err(e) => Err(e),
    }
    .context("Kpatch: Failed to read patch status")?;

    Ok(status)
}

fn write_patch_status(patch: &KernelPatch, value: &str) -> Result<()> {
    let sys_file = patch.sys_file.as_path();

    debug!("Writing '{}' to {}", value, sys_file.display());
    fs::write(sys_file, value).context("Kpatch: Failed to write patch status")
}

pub fn apply_patch(patch: &KernelPatch) -> Result<()> {
    debug!(
        "Inserting patch module '{}'...",
        patch.module_name.to_string_lossy()
    );
    let patch_module = fs::open_file(&patch.patch_file)?;
    kmod::finit_module(
        &patch_module,
        CString::new("")?.as_c_str(),
        kmod::ModuleInitFlags::MODULE_INIT_IGNORE_VERMAGIC,
    )
    .map_err(|e| anyhow!("Kpatch: {}", std::io::Error::from(e)))
}

pub fn remove_patch(patch: &KernelPatch) -> Result<()> {
    debug!(
        "Removing patch module '{}'...",
        patch.module_name.to_string_lossy()
    );

    kmod::delete_module(
        patch.module_name.to_cstring()?.as_c_str(),
        kmod::DeleteModuleFlags::O_NONBLOCK,
    )
    .map_err(|e| anyhow!("Kpatch: {}", std::io::Error::from(e)))
}

pub fn active_patch(patch: &KernelPatch) -> Result<()> {
    self::write_patch_status(patch, KPATCH_STATUS_ENABLED)
}

pub fn deactive_patch(patch: &KernelPatch) -> Result<()> {
    self::write_patch_status(patch, KPATCH_STATUS_DISABLED)
}
