use crate::cli::{CliWorkDir, CliArguments};

use super::{PatchBuilder, PatchBuilderArguments, PatchBuilderArgumentsParser};
use super::{PatchInfo, PatchType};

use super::user_patch::UserPatchBuilder;
use super::kernel_patch::KernelPatchBuilder;

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_builder(patch_info: &PatchInfo) -> Box<dyn PatchBuilder> {
        match patch_info.get_patch_type() {
            PatchType::KernelPatch => Box::new(KernelPatchBuilder::new()),
            PatchType::UserPatch   => Box::new(UserPatchBuilder::new()),
        }
    }

    pub fn parse_args(patch_info: &PatchInfo, work_dir: &CliWorkDir, args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        match patch_info.get_patch_type() {
            PatchType::KernelPatch => KernelPatchBuilder::parse_args(patch_info, work_dir, args),
            PatchType::UserPatch   => UserPatchBuilder::parse_args(patch_info, work_dir, args),
        }
    }
}
