use clap::Parser;

use crate::constants::*;
use crate::util::sys;

#[derive(Parser, Debug)]
#[command(bin_name=CLI_COMMAND_NAME, version)]
pub struct CliArguments {
    /// Patch name
    #[arg(short='n', long)]
    pub patch_name: String,

    /// Patch architecture
    #[arg(long, default_value=CLI_DEFAULT_PATCH_ARCH)]
    pub patch_arch: String,

    /// Patch version
    #[arg(long, default_value=CLI_DEFAULT_PATCH_VERSION)]
    pub patch_version: String,

    /// Patch description
    #[arg(long, default_value=CLI_DEFAULT_PATCH_DESCRIPTION)]
    pub patch_description: String,

    /// Patch target name
    #[arg(long)]
    pub target_name: Option<String>,

    /// Patch target executable name
    #[arg(short, long)]
    pub target_elfname: Option<String>,

    /// parch target architecture
    #[arg(long)]
    pub target_arch: Option<String>,

    /// Patch target epoch
    #[arg(long)]
    pub target_epoch: Option<String>,

    /// Patch target version
    #[arg(long)]
    pub target_version: Option<String>,

    /// Patch target release
    #[arg(long)]
    pub target_release: Option<String>,

    /// Patch target license
    #[arg(long)]
    pub target_license: Option<String>,

    /// Source package
    #[arg(short, long)]
    pub source: String,

    /// Debuginfo package
    #[arg(short, long)]
    pub debuginfo: String,

    /// Working directory
    #[arg(long, default_value=CLI_DEFAULT_WORKDIR)]
    pub workdir: String,

    /// Generated patch output directory
    #[arg(short, long, default_value=CLI_DEFAULT_OUTPUT_DIR)]
    pub output: String,

    /// Kernel make jobs
    #[arg(long, value_name="N", default_value=Self::get_default_kjobs())]
    pub kjobs: usize,

    /// Skip compiler version check (not recommended)
    #[arg(long, default_value=CLI_DEFAULT_SKIP_COMPILER_CHECK)]
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    #[arg(short, long, default_value=CLI_DEFAULT_VERBOSE_FLAG)]
    pub verbose: bool,

    /// Patch file(s)
    #[arg(required=true)]
    pub patches: Vec<String>
}

impl CliArguments {
    pub fn new() -> Self {
        CliArguments::parse()
    }

    fn get_default_kjobs() -> &'static str {
        Box::leak(
            sys::get_cpu_num()
                .to_string()
                .into_boxed_str()
        )
    }
}
