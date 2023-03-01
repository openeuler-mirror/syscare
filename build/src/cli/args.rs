use std::path::PathBuf;

use lazy_static::lazy_static;
use clap::Parser;

use crate::util::sys;

use super::{CLI_NAME, CLI_VERSION};

const DEFAULT_PATCH_VERSION:     &str = "1";
const DEFAULT_PATCH_DESCRIPTION: &str = "(none)";
const DEFAULT_WORK_DIR:          &str = ".";
const DEFAULT_OUTPUT_DIR:        &str = ".";

lazy_static! {
    static ref DEFAULT_KERNEL_JOBS: String = sys::cpu_num().to_string();
}

#[derive(Parser, Debug)]
#[command(bin_name=CLI_NAME, version=CLI_VERSION)]
pub struct CliArguments {
    /// Patch name
    #[arg(short='n', long)]
    pub patch_name: String,

    /// Patch architecture
    #[arg(long, default_value=sys::cpu_arch())]
    pub patch_arch: String,

    /// Patch version
    #[arg(long, default_value=DEFAULT_PATCH_VERSION)]
    pub patch_version: u32,

    /// Patch description
    #[arg(long, default_value=DEFAULT_PATCH_DESCRIPTION)]
    pub patch_description: String,

    /// Patch target name
    #[arg(long)]
    pub target_name: Option<String>,

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
    pub source: PathBuf,

    /// Debuginfo package
    #[arg(short, long)]
    pub debuginfo: PathBuf,

    /// Working directory
    #[arg(long, default_value=DEFAULT_WORK_DIR)]
    pub workdir: PathBuf,

    /// Generated patch output directory
    #[arg(short, long, default_value=DEFAULT_OUTPUT_DIR)]
    pub output: PathBuf,

    /// Kernel make jobs
    #[arg(long, value_name="N", default_value=DEFAULT_KERNEL_JOBS.as_str())]
    pub kjobs: usize,

    /// Skip compiler version check (not recommended)
    #[arg(long)]
    pub skip_compiler_check: bool,

    /// Skip post-build cleanup
    #[arg(long)]
    pub skip_cleanup: bool,

    /// Provide more detailed info
    #[arg(short, long)]
    pub verbose: bool,

    /// Patch file(s)
    #[arg(required=true)]
    pub patches: Vec<PathBuf>
}

impl CliArguments {
    pub fn new() -> Self {
        CliArguments::parse()
    }
}
