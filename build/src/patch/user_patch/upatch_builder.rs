use crate::cli::{CliWorkDir, CliArguments};

// use crate::package::RpmHelper;
use crate::patch::{PatchBuilder, PatchBuilderArguments, PatchInfo, PatchBuilderArgumentsParser};

// use crate::constants::*;

pub struct UserPatchBuilder;

impl UserPatchBuilder {
    pub fn new() -> Self {
        unimplemented!()
    }
}

impl PatchBuilderArgumentsParser for UserPatchBuilder {
    fn parse_args(_patch_info: &PatchInfo, _work_dir: &CliWorkDir, _args: &CliArguments) -> std::io::Result<PatchBuilderArguments> {
        unimplemented!()
    }
}

impl PatchBuilder for UserPatchBuilder {
    fn build_patch(&self, _args: PatchBuilderArguments) -> std::io::Result<()> {
        unimplemented!();
    }
}
