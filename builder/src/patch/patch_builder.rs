use syscare_abi::PatchInfo;

use crate::cli::CliArguments;

use super::patch_builder_args::PatchBuilderArguments;

pub trait PatchBuilder {
    fn parse_builder_args(
        &self,
        patch_info: &PatchInfo,
        args: &CliArguments,
    ) -> std::io::Result<PatchBuilderArguments>;
    fn build_patch(&self, args: &PatchBuilderArguments) -> std::io::Result<()>;
    fn write_patch_info(
        &self,
        patch_info: &mut PatchInfo,
        args: &PatchBuilderArguments,
    ) -> std::io::Result<()>;
}
