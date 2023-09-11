use anyhow::Result;

use syscare_abi::{PatchInfo, PatchType};

use crate::build_params::BuildParameters;
use crate::workdir::WorkDir;

use super::kernel_patch::{KernelPatchBuilder, KernelPatchBuilderArguments};
use super::user_patch::{UserPatchBuilder, UserPatchBuilderArguments};

pub enum PatchBuilderArguments {
    UserPatch(UserPatchBuilderArguments),
    KernelPatch(KernelPatchBuilderArguments),
}

pub trait PatchBuilder {
    fn parse_builder_args(&self, build_params: &BuildParameters) -> Result<PatchBuilderArguments>;
    fn build_patch(&self, args: &PatchBuilderArguments) -> Result<()>;
    fn generate_patch_info(
        &self,
        build_params: &BuildParameters,
        args: &PatchBuilderArguments,
    ) -> Result<Vec<PatchInfo>>;
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
