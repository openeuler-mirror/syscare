use crate::cli::CliWorkDir;

use syscare_abi::PatchType;

use super::kernel_patch::KernelPatchBuilder;
use super::user_patch::UserPatchBuilder;
use super::PatchBuilder;

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_builder(patch_type: PatchType, workdir: &CliWorkDir) -> Box<dyn PatchBuilder + '_> {
        match patch_type {
            PatchType::KernelPatch => Box::new(KernelPatchBuilder::new(workdir)),
            PatchType::UserPatch => Box::new(UserPatchBuilder::new(workdir)),
        }
    }
}
