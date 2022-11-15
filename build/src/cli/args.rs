use clap::Parser;

use crate::statics::*;

use super::CliPath;

#[derive(Parser, Debug)]
#[command(author, version, about="Syscare patch build utility")]
pub struct CliArguments {
    /// Patch name
    #[arg(short='n', long)]
    pub patch_name: String,

    /// Patch version
    #[arg(long, default_value=PATCH_DEFAULT_VERSION)]
    pub patch_version: String,

    /// Patch summary
    #[arg(long, default_value=PATCH_DEFAULT_SUMMARY)]
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
    pub debug_info: Option<CliPath>,

    /// Generated patch output directory
    #[arg(short, long, default_value=".")]
    pub output_dir: String,

    /// Kernel make config file
    #[arg(short, long)]
    pub kconfig: Option<String>,

    /// Kernel make jobs
    #[arg(long, value_name="N")]
    pub kjobs: Option<i32>,

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
    #[arg(long, default_value="false")]
    pub skip_compiler_check: bool,

    /// Patch file(s)
    #[arg(required=true)]
    pub patches: Vec<String>
}

impl CliArguments {
    pub fn new() -> Self {
        CliArguments::parse()
    }
}
