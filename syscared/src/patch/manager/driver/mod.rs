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

use anyhow::Result;

use syscare_abi::PatchStatus;

mod kpatch;
mod upatch;

pub use kpatch::*;
pub use upatch::*;

use super::entity::*;

#[derive(PartialEq, Clone, Copy)]
pub enum PatchOpFlag {
    Normal,
    Force,
}

/// Basic abstraction of patch operation
pub trait PatchDriver: Send + Sync {
    /// Perform file intergrity & consistency check. </br>
    /// Should be used befor patch application.
    fn check(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()>;

    /// Fetch and return the patch status.
    fn status(&self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus>;

    /// Apply a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    fn apply(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()>;

    /// Remove a patch. </br>
    /// After this action, the patch status would be changed to 'NOT-APPLIED'.
    fn remove(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()>;

    /// Active a patch. </br>
    /// After this action, the patch status would be changed to 'ACTIVED'.
    fn active(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()>;

    /// Deactive a patch. </br>
    /// After this action, the patch status would be changed to 'DEACTIVED'.
    fn deactive(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()>;
}
