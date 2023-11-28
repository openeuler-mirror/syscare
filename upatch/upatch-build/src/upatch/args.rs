use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::bail;
use clap::{AppSettings, ColorChoice, Parser};

use super::Result;
use crate::tool::*;

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

const DEFAULT_WORK_DIR: &str = "~/.upatch";
const DEFAULT_BUILD_PATCH_CMD: &str = "";
const DEFAULT_COMPILERS: &str = "gcc";
const DEFAULT_OUTPUT_DIR: &str = "~/.upatch";

#[derive(Parser, Debug)]
#[clap(
    bin_name = CLI_NAME,
    version = CLI_VERSION,
    about = CLI_ABOUT,
    arg_required_else_help(true),
    color(ColorChoice::Never),
    global_setting(AppSettings::DeriveDisplayOrder),
    term_width(120),
)]
pub struct Arguments {
    /// Specify output name
    #[clap(short, long, default_value = "", hide_default_value = true)]
    pub name: OsString,

    /// Specify working directory
    #[clap(short, long, default_value = DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    /// Specify source directory
    #[clap(short, long)]
    pub source_dir: PathBuf,

    /// Specify build source command
    #[clap(short, long)]
    pub build_source_cmd: String,

    /// Specify build patched source command [default: <BUILD_SOURCE_CMD>]
    #[clap(long, default_value = DEFAULT_BUILD_PATCH_CMD, hide_default_value = true)]
    pub build_patch_cmd: String,

    /// Specify debuginfo files
    #[clap(short, long, multiple = true, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Specify the directory of searching elf [default: <SOURCE_DIR>]
    #[clap(long, required = false)]
    pub elf_dir: Option<PathBuf>,

    /// Specify elf's relative path relate to 'elf_dir' or absolute patch list
    #[clap(long = "elf-path", multiple = true, required = true)]
    pub elf_path: Vec<PathBuf>,

    /// Specify compiler(s)
    #[clap(short, long,  multiple = true, default_value = DEFAULT_COMPILERS)]
    pub compiler: Vec<PathBuf>,

    /// Patch file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub patch: Vec<PathBuf>,

    /// Specify output directory [default: <WORK_DIR>]
    #[clap(short, long, default_value = DEFAULT_OUTPUT_DIR, hide_default_value = true)]
    pub output_dir: PathBuf,

    /// Skip compiler version check (not recommended)
    #[clap(long)]
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse()
            .check()
            .map_err(|e| super::Error::Mod(e.to_string()))
    }

    fn check(mut self) -> anyhow::Result<Self> {
        if !self.work_dir.is_dir() {
            bail!(
                "Working directory \"{}\" should be a directory",
                self.work_dir.display()
            );
        }
        self.work_dir = real_arg(self.work_dir)?.join("upatch");

        if !self.source_dir.is_dir() {
            bail!(
                "Source directory \"{}\" should be a directory",
                self.source_dir.display()
            );
        }
        self.source_dir = real_arg(&self.source_dir)?;

        for debuginfo in &mut self.debuginfo {
            if !debuginfo.is_file() {
                bail!("Debuginfo \"{}\" should be a file", debuginfo.display());
            }
            *debuginfo = real_arg(&debuginfo)?;
        }

        for patch in &mut self.patch {
            if !patch.is_file() {
                bail!("Patch \"{}\" should be a file", patch.display());
            }
            *patch = real_arg(&patch)?;
        }

        if self.build_patch_cmd.is_empty() {
            self.build_patch_cmd = self.build_source_cmd.clone();
        }

        if !self.name.is_empty() {
            self.name.push("-");
        }

        self.elf_dir = match &self.elf_dir {
            Some(elf_dir) => Some({
                if !elf_dir.is_dir() {
                    bail!(
                        "Elf directory \"{}\" should be a directory",
                        elf_dir.display()
                    );
                }
                real_arg(elf_dir)?
            }),
            None => Some(self.source_dir.clone()),
        };

        match self.elf_path.len().eq(&self.debuginfo.len()) {
            true => {
                let elf_dir = self.elf_dir.as_ref().unwrap();
                for elf_path in &mut self.elf_path {
                    *elf_path = elf_dir.join(&elf_path);
                }
            }
            false => {
                bail!(
                    "{}'s elf-path don't match {}'s debug-info",
                    self.elf_path.len(),
                    self.debuginfo.len()
                );
            }
        }

        if !self.output_dir.is_dir() {
            bail!(
                "Output directory \"{}\" should be a directory",
                self.output_dir.display()
            );
        }

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
