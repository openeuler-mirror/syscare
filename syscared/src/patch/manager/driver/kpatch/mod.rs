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

use std::{ffi::OsString, os::unix::prelude::OsStrExt, path::Path};

use anyhow::{anyhow, bail, ensure, Context, Result};
use lazy_static::lazy_static;
use log::debug;

use syscare_abi::PatchStatus;
use syscare_common::{
    os,
    util::{
        digest,
        ext_cmd::{ExternCommand, ExternCommandArgs},
        fs,
        os_str::OsStringExt,
    },
};

use super::{KernelPatchExt, Patch, PatchDriver, PatchOpFlag};

lazy_static! {
    static ref INSMOD: ExternCommand = ExternCommand::new("insmod");
    static ref RMMOD: ExternCommand = ExternCommand::new("rmmod");
}

const KPATCH_PATCH_SEC_TYPE: &str = "modules_object_t";
const KPATCH_STATUS_DISABLED: &str = "0";
const KPATCH_STATUS_ENABLED: &str = "1";

pub struct KernelPatchDriver;

impl KernelPatchDriver {
    fn set_patch_security_context<P: AsRef<Path>>(patch_file: P) -> Result<()> {
        if os::selinux::get_enforce()? != os::selinux::SELinuxStatus::Enforcing {
            debug!("SELinux is disabled");
            return Ok(());
        }
        debug!("SELinux is enforcing");

        let file_path = patch_file.as_ref();
        if os::selinux::get_security_context_type(file_path)? != KPATCH_PATCH_SEC_TYPE {
            os::selinux::set_security_context_type(file_path, KPATCH_PATCH_SEC_TYPE)?;
        }

        Ok(())
    }

    fn get_patch_status(patch: &Patch) -> Result<PatchStatus> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let sys_file = patch_ext.sys_file.as_path();

        debug!("Reading \"{}\"", sys_file.display());
        let status = match fs::read_to_string(sys_file) {
            Ok(str) => {
                let status = str.trim();
                let patch_status: PatchStatus = match status {
                    KPATCH_STATUS_DISABLED => PatchStatus::Deactived,
                    KPATCH_STATUS_ENABLED => PatchStatus::Actived,
                    _ => {
                        bail!("Kpatch: Patch status \"{}\" is invalid", status);
                    }
                };
                Ok(patch_status)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PatchStatus::NotApplied),
            Err(e) => Err(e),
        }
        .with_context(|| format!("Kpatch: Failed to read patch \"{}\" status", patch))?;

        Ok(status)
    }

    fn set_patch_status(patch: &Patch, status: PatchStatus) -> Result<()> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let sys_file = patch_ext.sys_file.as_path();

        let status_str = match status {
            PatchStatus::NotApplied | PatchStatus::Deactived => KPATCH_STATUS_DISABLED,
            PatchStatus::Actived => KPATCH_STATUS_ENABLED,
            _ => bail!("Kpatch: Patch status \"{}\" is invalid", status),
        };

        debug!("Writing \"{}\" to \"{}\"", status_str, sys_file.display());
        fs::write(sys_file, status_str)
            .with_context(|| format!("Kpatch: Failed to write patch \"{}\" status", patch))?;

        Ok(())
    }
}

impl KernelPatchDriver {
    fn check_compatiblity(&self, patch: &Patch) -> Result<()> {
        const KERNEL_NAME_PREFIX: &str = "kernel-";

        let kernel_version = os::kernel::version();
        let current_kernel = OsString::from(KERNEL_NAME_PREFIX).concat(kernel_version);

        let patch_target = patch.target_pkg_name.clone();
        debug!("Patch target:   \"{}\"", patch_target);
        debug!("Current kernel: \"{}\"", current_kernel.to_string_lossy());

        if patch_target.starts_with(KERNEL_NAME_PREFIX)
            && (patch_target.as_bytes() != current_kernel.as_bytes())
        {
            bail!(
                "Kpatch: Current kernel \"{}\" is incompatible with patch target \"{}\"",
                kernel_version.to_string_lossy(),
                patch_target
            );
        }

        Ok(())
    }

    fn check_consistency(&self, patch: &Patch) -> Result<()> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();
        let real_checksum = digest::file(patch_file)?;
        debug!("Target checksum: {}", patch.checksum);
        debug!("Expected checksum: {}", real_checksum);

        ensure!(
            patch.checksum.eq(&real_checksum),
            "Kpatch: Patch \"{}\" consistency check failed",
            patch_file.display()
        );

        Ok(())
    }

    fn check_confliction(&self, _patch: &Patch) -> Result<()> {
        Ok(())
    }
}

impl PatchDriver for KernelPatchDriver {
    fn check(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.check_compatiblity(patch)?;
        self.check_consistency(patch)?;

        if flag != PatchOpFlag::Force {
            self.check_confliction(patch)?;
        }

        Ok(())
    }

    fn status(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<PatchStatus> {
        Self::get_patch_status(patch)
    }

    fn apply(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();

        Self::set_patch_security_context(patch_file)
            .context("Kpatch: Failed to set patch security context")?;

        let exit_status = INSMOD.execvp(ExternCommandArgs::new().arg(patch_file))?;
        exit_status.check_exit_code().map_err(|_| {
            anyhow!(
                "Kpatch: Failed to insert patch module, exit_code={}",
                exit_status.exit_code()
            )
        })?;

        Ok(())
    }

    fn remove(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();

        let exit_status = RMMOD.execvp(ExternCommandArgs::new().arg(patch_file))?;
        exit_status.check_exit_code().map_err(|_| {
            anyhow!(
                "Kpatch: Failed to remove patch module, exit_code={}",
                exit_status.exit_code()
            )
        })?;

        Ok(())
    }

    fn active(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        Self::set_patch_status(patch, PatchStatus::Actived)
    }

    fn deactive(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        Self::set_patch_status(patch, PatchStatus::Deactived)
    }
}