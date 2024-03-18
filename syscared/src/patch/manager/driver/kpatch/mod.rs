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

use std::{ffi::OsString, os::unix::ffi::OsStrExt, path::Path};

use anyhow::{bail, ensure, Context, Result};
use log::debug;

use syscare_abi::PatchStatus;
use syscare_common::{
    concat_os, fs,
    os::{self, selinux},
    process::Command,
    util::digest,
};

use super::{KernelPatchExt, Patch, PatchDriver, PatchOpFlag};

const INSMOD_BIN: &str = "insmod";
const RMMOD_BIN: &str = "rmmod";

const KPATCH_PATCH_SEC_TYPE: &str = "modules_object_t";
const KPATCH_STATUS_DISABLED: &str = "0";
const KPATCH_STATUS_ENABLED: &str = "1";

pub struct KernelPatchDriver;

impl KernelPatchDriver {
    fn set_patch_security_context<P: AsRef<Path>>(patch_file: P) -> Result<()> {
        if selinux::get_status()? != selinux::Status::Enforcing {
            debug!("SELinux is disabled");
            return Ok(());
        }
        debug!("SELinux is enforcing");

        let file_path = patch_file.as_ref();
        let mut sec_context = selinux::get_security_context(file_path)?;

        if sec_context.kind != KPATCH_PATCH_SEC_TYPE {
            sec_context.kind = OsString::from(KPATCH_PATCH_SEC_TYPE);
            selinux::set_security_context(file_path, sec_context)?;
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
        let current_kernel = concat_os!(KERNEL_NAME_PREFIX, kernel_version);

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

        Command::new(INSMOD_BIN)
            .arg(patch_file)
            .run()?
            .exit_ok()
            .context("Kpatch: Failed to insert patch module")?;

        Ok(())
    }

    fn remove(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &KernelPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();

        Command::new(RMMOD_BIN)
            .arg(patch_file)
            .run()?
            .exit_ok()
            .context("Kpatch: Failed to remove patch module")?;

        Ok(())
    }

    fn active(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        Self::set_patch_status(patch, PatchStatus::Actived)
    }

    fn deactive(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        Self::set_patch_status(patch, PatchStatus::Deactived)
    }
}
