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

use super::entity::Patch;

pub struct PatchDriver {
    kpatch: KernelPatchDriver,
    upatch: UserPatchDriver,
}

impl PatchDriver {
    pub fn new() -> Result<Self> {
        info!("Initializing kernel patch driver...");
        let kpatch_driver =
            KernelPatchDriver::new().context("Failed to initialize kernel patch driver")?;

        info!("Initializing user patch driver...");
        let upatch_driver =
            UserPatchDriver::new().context("Failed to initialize user patch driver")?;

        Ok(Self {
            kpatch: kpatch_driver,
            upatch: upatch_driver,
        })
    }

    /// Fetch and return the patch status.
    pub fn status(&self, patch: &Patch) -> Result<PatchStatus> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.status(patch),
            Patch::UserPatch(patch) => self.upatch.status(patch),
        }
    }

    /// Perform file intergrity & consistency check. </br>
    /// Should be used befor patch application.
    pub fn check(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.check(patch, flag),
            Patch::UserPatch(patch) => self.upatch.check(patch, flag),
        }
        .with_context(|| format!("Patch '{}' check failed", patch))
    }

    /// Apply a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn apply(&mut self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.apply(patch),
            Patch::UserPatch(patch) => self.upatch.apply(patch),
        }
        .with_context(|| format!("Failed to apply patch '{}'", patch))
    }

    /// Remove a patch. </br>
    /// After this action, the patch status would be changed to 'NOT-APPLIED'.
    pub fn remove(&mut self, patch: &Patch) -> Result<()> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.remove(patch),
            Patch::UserPatch(patch) => self.upatch.remove(patch),
        }
        .with_context(|| format!("Failed to remove patch '{}'", patch))
    }

    /// Active a patch. </br>
    /// After this action, the patch status would be changed to 'ACTIVED'.
    pub fn active(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.active(patch, flag),
            Patch::UserPatch(patch) => self.upatch.active(patch, flag),
        }
        .with_context(|| format!("Failed to active patch '{}'", patch))
    }

    /// Deactive a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    pub fn deactive(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        match patch {
            Patch::KernelPatch(patch) => self.kpatch.deactive(patch, flag),
            Patch::UserPatch(patch) => self.upatch.deactive(patch, flag),
        }
        .with_context(|| format!("Failed to deactive patch '{}'", patch))
    }
}
