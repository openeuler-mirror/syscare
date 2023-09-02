use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use lazy_static::lazy_static;

use syscare_common::{os, util::fs};

use crate::{CLI_NAME, CLI_VERSION};

const DEFAULT_PATCH_VERSION: &str = "1";
const DEFAULT_PATCH_RELEASE: &str = "1";
const DEFAULT_PATCH_DESCRIPTION: &str = "(none)";
const DEFAULT_WORK_DIR: &str = ".";
const DEFAULT_OUTPUT_DIR: &str = ".";

lazy_static! {
    static ref DEFAULT_BUILD_JOBS: String = os::cpu::num().to_string();
    static ref DEFAULT_PATCH_ARCH: String = os::cpu::arch().to_string_lossy().to_string();
}

#[derive(Parser, Debug)]
#[clap(bin_name=CLI_NAME, version=CLI_VERSION)]
pub struct Arguments {
    /// Patch name
    #[clap(short = 'n', long)]
    pub patch_name: String,

    /// Patch architecture
    #[clap(long, default_value=DEFAULT_PATCH_ARCH.as_str())]
    pub patch_arch: String,

    /// Patch version
    #[clap(long, default_value=DEFAULT_PATCH_VERSION)]
    pub patch_version: String,

    /// Patch release
    #[clap(long, default_value=DEFAULT_PATCH_RELEASE)]
    pub patch_release: u32,

    /// Patch description
    #[clap(long, default_value=DEFAULT_PATCH_DESCRIPTION)]
    pub patch_description: String,

    /// Source package
    #[clap(short, long)]
    pub source: PathBuf,

    /// Debuginfo package(s)
    #[clap(short, long, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Working directory
    #[clap(long, default_value=DEFAULT_WORK_DIR)]
    pub workdir: PathBuf,

    /// Generated patch output directory
    #[clap(short, long, default_value=DEFAULT_OUTPUT_DIR)]
    pub output: PathBuf,

    /// Parallel build jobs
    #[clap(short, long, value_name="N", default_value=DEFAULT_BUILD_JOBS.as_str())]
    pub jobs: usize,

    /// Skip compiler version check (not recommended)
    #[clap(long)]
    pub skip_compiler_check: bool,

    /// Skip post-build cleanup
    #[clap(long)]
    pub skip_cleanup: bool,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,

    /// Patch file(s)
    #[clap(required = true)]
    pub patches: Vec<PathBuf>,
}

impl Arguments {
    pub fn new() -> Self {
        Arguments::parse()
            .normalize_pathes()
            .expect("Failed to parse arguments")
    }

    fn normalize_pathes(mut self) -> Result<Self> {
        self.source = fs::normalize(&self.source)?;
        for debuginfo in &mut self.debuginfo {
            *debuginfo = fs::normalize(&debuginfo)?
        }
        self.workdir = fs::normalize(&self.workdir)?;
        self.output = fs::normalize(&self.output)?;
        for patch in &mut self.patches {
            *patch = fs::normalize(&patch)?
        }

        Ok(self)
    }
}

impl Default for Arguments {
    fn default() -> Self {
        Self::new()
    }
}
