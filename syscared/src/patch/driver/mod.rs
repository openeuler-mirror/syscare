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

use anyhow::{Context, Result};

use log::info;
use syscare_abi::PatchStatus;

mod kpatch;
mod upatch;

pub use kpatch::*;
pub use upatch::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchOpFlag {
    Normal,
    Force,
}

use crate::config::PatchConfig;

use super::entity::Patch;

pub struct PatchDriver {
    kpatch: KernelPatchDriver,
    upatch: UserPatchDriver,
}

impl PatchDriver {
    fn check_conflict_functions(&self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check_conflict_functions(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check_conflict_functions(upatch),
        }
    }

    fn check_override_functions(&self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check_override_functions(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check_override_functions(upatch),
        }
    }
}

impl PatchDriver {
    pub fn new(config: &PatchConfig) -> Result<Self> {
        info!("Initializing kernel patch driver...");
        let kpatch_driver = KernelPatchDriver::new(&config.kpatch)
            .context("Failed to initialize kernel patch driver")?;

        info!("Initializing user patch driver...");
        let upatch_driver = UserPatchDriver::new(&config.upatch)
            .context("Failed to initialize user patch driver")?;

        Ok(Self {
            kpatch: kpatch_driver,
            upatch: upatch_driver,
        })
    }

    /// Fetch and return the patch status.
    pub fn patch_status(&self, patch: &Patch) -> Result<PatchStatus> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.status(kpatch),
            Patch::UserPatch(upatch) => self.upatch.status(upatch),
        }
        .with_context(|| format!("Failed to get patch '{}' status", patch))
    }

    /// Perform patch file intergrity & consistency check. </br>
    /// Should be used before patch application.
    pub fn check_patch(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        if flag == PatchOpFlag::Force {
            return Ok(());
        }
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check(upatch),
        }
        .with_context(|| format!("Patch '{}' is not patchable", patch))
    }

    /// Perform patch confliction check. </br>
    /// Used for patch check.
    pub fn check_confliction(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        if flag == PatchOpFlag::Force {
            return Ok(());
        }
        self.check_conflict_functions(patch)
            .with_context(|| format!("Patch '{}' is conflicted", patch))
    }

    /// Apply a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn apply_patch(&mut self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.apply(kpatch),
            Patch::UserPatch(upatch) => self.upatch.apply(upatch),
        }
        .with_context(|| format!("Failed to apply patch '{}'", patch))
    }

    /// Remove a patch. </br>
    /// After this action, the patch status would be changed to 'NOT-APPLIED'.
    pub fn remove_patch(&mut self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.remove(kpatch),
            Patch::UserPatch(upatch) => self.upatch.remove(upatch),
        }
        .with_context(|| format!("Failed to remove patch '{}'", patch))
    }

    /// Active a patch. </br>
    /// After this action, the patch status would be changed to 'ACTIVED'.
    pub fn active_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        if flag != PatchOpFlag::Force {
            self.check_conflict_functions(patch)?;
        }
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.active(kpatch),
            Patch::UserPatch(upatch) => self.upatch.active(upatch),
        }
        .with_context(|| format!("Failed to active patch '{}'", patch))
    }

    /// Deactive a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn deactive_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        if flag != PatchOpFlag::Force {
            self.check_override_functions(patch)?;
        }
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.deactive(kpatch),
            Patch::UserPatch(upatch) => self.upatch.deactive(upatch),
        }
        .with_context(|| format!("Failed to deactive patch '{}'", patch))
    }
}
