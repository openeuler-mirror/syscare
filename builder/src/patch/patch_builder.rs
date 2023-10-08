use anyhow::Result;

use syscare_abi::{PatchInfo, PatchType};

use crate::build_params::BuildParameters;

use super::{kernel_patch::KernelPatchBuilder, user_patch::UserPatchBuilder};

pub trait PatchBuilder {
    fn build_patch(&self, build_params: &BuildParameters) -> Result<Vec<PatchInfo>>;
}

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_builder(patch_type: PatchType) -> Box<dyn PatchBuilder> {
        match patch_type {
            PatchType::KernelPatch => Box::new(KernelPatchBuilder),
            PatchType::UserPatch => Box::new(UserPatchBuilder),
        }
    }
}
