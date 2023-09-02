use anyhow::Result;

use syscare_abi::{PatchInfo, PatchType};

use crate::args::Arguments;
use crate::workdir::WorkDir;

use super::kernel_patch::{KernelPatchBuilder, KernelPatchBuilderArguments};
use super::user_patch::{UserPatchBuilder, UserPatchBuilderArguments};

pub enum PatchBuilderArguments {
    UserPatch(UserPatchBuilderArguments),
    KernelPatch(KernelPatchBuilderArguments),
}

pub trait PatchBuilder {
    fn parse_builder_args(
        &self,
        patch_info: &PatchInfo,
        args: &Arguments,
    ) -> Result<PatchBuilderArguments>;
    fn build_patch(&self, args: &PatchBuilderArguments) -> Result<()>;
    fn write_patch_info(
        &self,
        patch_info: &mut PatchInfo,
        args: &PatchBuilderArguments,
    ) -> Result<()>;
}

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_builder(patch_type: PatchType, workdir: &WorkDir) -> Box<dyn PatchBuilder + '_> {
        match patch_type {
            PatchType::KernelPatch => Box::new(KernelPatchBuilder::new(workdir)),
            PatchType::UserPatch => Box::new(UserPatchBuilder::new(workdir)),
        }
    }
}
