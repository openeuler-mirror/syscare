use clap::Parser;

use crate::constants::*;
use crate::util::sys;

use super::CliPath;

#[derive(Parser, Debug)]
#[command(author, version, about="Syscare patch build utility")]
pub struct CliArguments {
    /// Patch name
    #[arg(short='n', long)]
    pub patch_name: String,

    /// Patch version
    #[arg(long, default_value=CLI_DEFAULT_PATCH_VERSION)]
    pub patch_version: String,

    /// Patch summary
    #[arg(long, default_value=CLI_DEFAULT_PATCH_SUMMARY)]
    pub patch_summary: String,

    /// Patch target name
    #[arg(long)]
    pub target_name: Option<String>,

    /// Patch target version
    #[arg(long)]
    pub target_version: Option<String>,

    /// Patch target release
    #[arg(long)]
    pub target_release: Option<String>,

    /// Patch target license
    #[arg(long)]
    pub target_license: Option<String>,

    /// Source directory or source package
    #[arg(short, long)]
    pub source: CliPath,

    /// Debug info (vmlinux for kernel)
    #[arg(short, long)]
    pub debug_info: Option<String>,

    /// Working directory
    #[arg(long, default_value=CLI_DEFAULT_WORK_DIR)]
    pub work_dir: String,

    /// Generated patch output directory
    #[arg(short, long, default_value=CLI_DEFAULT_OUTPUT_DIR)]
    pub output_dir: String,

    /// Kernel make config file
    #[arg(short, long)]
    pub kconfig: Option<String>,

    /// Kernel make jobs
    #[arg(long, value_name="N", default_value=Self::get_default_kjobs())]
    pub kjobs: usize,

    // /// Kernel make targets, split by ','
    // #[arg(long, value_delimiter=',')]
    // pub ktarget: Option<Vec<String>>,

    // /// Kernel module make directory
    // #[arg(long)]
    // pub kmod_dir: Option<String>,

    // /// Kernel module make flags
    // #[arg(long)]
    // pub kmod_flag: Option<String>,

    /// User build command
    #[arg(short, long)]
    pub build_entry: Option<String>,

    /// Skip compiler version check (not recommended)
    #[arg(long, default_value=CLI_DEFAULT_SKIP_COMPILER_CHECK)]
    pub skip_compiler_check: bool,

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
