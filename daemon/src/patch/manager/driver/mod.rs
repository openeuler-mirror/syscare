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
    SkipCheck,
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
