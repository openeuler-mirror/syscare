use std::path::PathBuf;

use clap::Parser;
use lazy_static::lazy_static;

use common::os;

use super::PatchBuildCLI;

const DEFAULT_PATCH_VERSION: &str = "1";
const DEFAULT_PATCH_RELEASE: &str = "1";
const DEFAULT_PATCH_DESCRIPTION: &str = "(none)";
const DEFAULT_WORK_DIR: &str = ".";
const DEFAULT_OUTPUT_DIR: &str = ".";

lazy_static! {
    static ref DEFAULT_BUILD_JOBS: String = os::cpu::num().to_string();
}

#[derive(Parser, Debug)]
#[command(bin_name=PatchBuildCLI::name(), version=PatchBuildCLI::version())]
pub struct CliArguments {
    /// Patch name
    #[arg(short = 'n', long)]
    pub patch_name: String,

    /// Patch architecture
    #[arg(long, default_value=os::cpu::arch())]
    pub patch_arch: String,

    /// Patch version
    #[arg(long, default_value=DEFAULT_PATCH_VERSION)]
    pub patch_version: String,

    /// Patch release
    #[arg(long, default_value=DEFAULT_PATCH_RELEASE)]
    pub patch_release: u32,

    /// Patch description
    #[arg(long, default_value=DEFAULT_PATCH_DESCRIPTION)]
    pub patch_description: String,

    /// Source package
    #[arg(short, long)]
    pub source: PathBuf,

    /// Debuginfo package(s)
    #[arg(short, long, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Working directory
    #[arg(long, default_value=DEFAULT_WORK_DIR)]
    pub workdir: PathBuf,

    /// Generated patch output directory
    #[arg(short, long, default_value=DEFAULT_OUTPUT_DIR)]
    pub output: PathBuf,

    /// Parallel build jobs
    #[arg(short, long, value_name="N", default_value=DEFAULT_BUILD_JOBS.as_str())]
    pub jobs: usize,

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
    #[arg(required = true)]
    pub patches: Vec<PathBuf>,
}

impl CliArguments {
    pub fn new() -> Self {
        CliArguments::parse()
    }
}

impl Default for CliArguments {
    fn default() -> Self {
        Self::new()
    }
}
