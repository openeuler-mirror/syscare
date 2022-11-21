use crate::cli::{CliWorkDir, CliArguments};

use super::patch_builder_args::PatchBuilderArguments;
use super::patch_info::PatchInfo;

pub trait PatchBuilder {
    fn build_patch(&self, options: PatchBuilderArguments) -> std::io::Result<()>;
}

pub trait PatchBuilderArgumentsParser {
    fn parse_args(patch_info: &PatchInfo, workdir: &CliWorkDir, args: &CliArguments) -> std::io::Result<PatchBuilderArguments>;
}
