use std::path::PathBuf;
use std::ffi::OsString;

use clap::Parser;
use which::which;

use crate::tool::*;

#[derive(Parser, Debug)]
#[command(bin_name="upatch-build", version, term_width = 200)]
pub struct Arguments {
    /// Specify work directory
    /// will delete the work_dir [default: ~/.upatch]
    #[arg(short, long, default_value = None, verbatim_doc_comment)]
    pub work_dir: Option<PathBuf>,

    /// Specify source directory
    /// will modify the debug_source
    #[arg(short = 's', long, verbatim_doc_comment)]
    pub debug_source: PathBuf,

    /// Specify build source command
    #[arg(short, long)]
    pub build_source_cmd: String,

    /// Specify build patched command [default: <BUILD_SOURCE_COMMAND>]
    #[arg(long, default_value_t = String::new(), hide_default_value = true)]
    pub build_patch_cmd: String,

    /// Specify debug info array
    #[arg(short = 'i', long = "debug-info", required = true)]
    pub debug_infoes: Vec<PathBuf>,

    /// Specify compiler [default: gcc]
    #[arg(short, long, default_value = None)]
    pub compiler: Option<PathBuf>,

    /// Specify output directory [default: <WORK_DIR>]
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Specify output name
    #[arg(short, long, default_value = "", hide_default_value = true)]
    pub name: OsString,

    /// Skip compiler version check (not recommended)
    #[arg(long, default_value = "false")]
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Patch file(s)
    #[arg(required = true)]
    pub patches: Vec<PathBuf>
}

impl Arguments {
    pub fn new() -> Self {
        Arguments::parse()
    }
}

impl Arguments {
    pub fn check(&mut self) -> std::io::Result<()> {
        self.work_dir = match &self.work_dir {
            Some(work_dir) => Some(real_arg(work_dir)?),
            #[allow(deprecated)]
            None => Some(match std::env::home_dir() {
                Some(work_dir) => work_dir,
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("home_dir don't support BSD system"),
                )),
            }),
        };

        self.compiler = match &self.compiler {
            Some(compiler) => Some(real_arg(compiler)?),
            None => Some(match which("gcc") {
                        Ok(compiler) => compiler,
                        Err(e) => return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("can't find gcc in system: {}", e),
                        )),
                    }),
        };

        self.debug_source = real_arg(&self.debug_source)?;

        for debug_info in &mut self.debug_infoes {
            *debug_info = real_arg(&debug_info)?;
        }

        for patch in &mut self.patches {
            *patch = real_arg(&patch)?;
        }

        if self.build_patch_cmd.is_empty() {
            self.build_patch_cmd = self.build_source_cmd.clone();
        }

        if !self.name.is_empty() {
            self.name.push("-");
        }

        Ok(())
    }
}