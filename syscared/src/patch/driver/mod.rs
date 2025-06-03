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

use log::{debug, info};
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
    fn check_conflicted_patches(&self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check_conflicted_patches(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check_conflicted_patches(upatch),
        }
    }

    fn check_overridden_patches(&self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check_overridden_patches(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check_overridden_patches(upatch),
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

    /// Perform patch confliction check. </br>
    /// Used for patch check.
    pub fn check_confliction(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        if flag == PatchOpFlag::Force {
            return Ok(());
        }
        self.check_conflicted_patches(patch)
            .with_context(|| format!("Patch '{}' is conflicted", patch))
    }

    /// Perform patch file intergrity & consistency check. </br>
    /// Should be used before patch application.
    pub fn check_patch(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        info!("Checking patch '{}'...", patch);

        if flag == PatchOpFlag::Force {
            return Ok(());
        }
        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.check_patch(kpatch),
            Patch::UserPatch(upatch) => self.upatch.check_patch(upatch),
        }
        .with_context(|| format!("Patch '{}' is not patchable", patch))
    }

    /// Fetch and return the patch status.
    pub fn get_patch_status(&self, patch: &Patch) -> Result<PatchStatus> {
        debug!("Fetching patch '{}' status...", patch);

        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.get_patch_status(kpatch),
            Patch::UserPatch(upatch) => self.upatch.get_patch_status(upatch),
        }
        .with_context(|| format!("Failed to get patch '{}' status", patch))
    }

    /// Load a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn load_patch(&mut self, patch: &Patch) -> Result<()> {
        info!("Loading patch '{}'...", patch);

        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.load_patch(kpatch),
            Patch::UserPatch(upatch) => self.upatch.load_patch(upatch),
        }
        .with_context(|| format!("Failed to load patch '{}'", patch))
    }

    /// Remove a patch. </br>
    /// After this action, the patch status would be changed to 'NOT-APPLIED'.
    pub fn remove_patch(&mut self, patch: &Patch) -> Result<()> {
        info!("Removing patch '{}'...", patch);

        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.remove_patch(kpatch),
            Patch::UserPatch(upatch) => self.upatch.remove_patch(upatch),
        }
        .with_context(|| format!("Failed to remove patch '{}'", patch))
    }

    /// Active a patch. </br>
    /// After this action, the patch status would be changed to 'ACTIVED'.
    pub fn active_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        info!("Activating patch '{}'...", patch);

        if flag != PatchOpFlag::Force {
            self.check_conflicted_patches(patch)?;
        }

        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.active_patch(kpatch),
            Patch::UserPatch(upatch) => self.upatch.active_patch(upatch),
        }
        .with_context(|| format!("Failed to active patch '{}'", patch))
    }

    /// Deactive a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn deactive_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        info!("Deactivating patch '{}'...", patch);

        if flag != PatchOpFlag::Force {
            self.check_overridden_patches(patch)?;
        }

        match patch {
            Patch::KernelPatch(kpatch) => self.kpatch.deactive_patch(kpatch),
            Patch::UserPatch(upatch) => self.upatch.deactive_patch(upatch),
        }
        .with_context(|| format!("Failed to deactive patch '{}'", patch))
    }
}
