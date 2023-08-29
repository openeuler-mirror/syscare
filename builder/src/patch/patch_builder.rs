use anyhow::Result;
use syscare_abi::PatchInfo;

use crate::args::Arguments;

use super::patch_builder_args::PatchBuilderArguments;

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
