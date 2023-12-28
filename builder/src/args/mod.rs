use std::path::PathBuf;

use anyhow::{bail, ensure, Result};
use clap::ArgMatches;

use syscare_common::{os, util::fs};

mod matcher;
mod parser;

use matcher::ArgMatcher;
use parser::{ArgParser, ArgParserImpl, Parser};

#[derive(Debug)]
pub struct Arguments {
    /// Patch name
    pub patch_name: String,

    /// Patch architecture
    pub patch_arch: String,

    /// Patch version
    pub patch_version: String,

    /// Patch release
    pub patch_release: u32,

    /// Patch description
    pub patch_description: String,

    /// Patch requires
    pub patch_requires: Vec<String>,

    /// Source package
    pub source: Vec<PathBuf>,

    /// Debuginfo package(s)
    pub debuginfo: Vec<PathBuf>,

    /// Patch file(s)
    pub patch: Vec<PathBuf>,

    /// Working directory
    pub work_dir: PathBuf,

    /// Build temporary directory
    pub build_root: PathBuf,

    /// Generated patch output directory
    pub output: PathBuf,

    /// Parallel build jobs
    pub jobs: usize,

    /// Skip compiler version check (not recommended)
    pub skip_compiler_check: bool,

    /// Skip post-build cleanup
    pub skip_cleanup: bool,

    /// Provide more detailed info
    pub verbose: bool,
}

impl Parser<'_> for Arguments {
    fn parse(matches: &ArgMatches<'_>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            patch_name: ArgParserImpl::parse_arg(matches, "patch_name")?,
            patch_arch: ArgParserImpl::parse_arg(matches, "patch_arch")?,
            patch_version: ArgParserImpl::parse_arg(matches, "patch_version")?,
            patch_release: ArgParserImpl::parse_arg(matches, "patch_release")?,
            patch_description: ArgParserImpl::parse_arg(matches, "patch_description")?,
            patch_requires: ArgParserImpl::parse_args(matches, "patch_requires")?,
            source: ArgParserImpl::parse_args(matches, "source")?,
            debuginfo: ArgParserImpl::parse_args(matches, "debuginfo")?,
            patch: ArgParserImpl::parse_args(matches, "patch")?,
            work_dir: ArgParserImpl::parse_arg(matches, "work_dir")?,
            build_root: ArgParserImpl::parse_arg(matches, "build_root")?,
            output: ArgParserImpl::parse_arg(matches, "output")?,
            jobs: ArgParserImpl::parse_arg(matches, "jobs")?,
            skip_compiler_check: ArgParserImpl::is_present(matches, "skip_compiler_check"),
            skip_cleanup: ArgParserImpl::is_present(matches, "skip_cleanup"),
            verbose: ArgParserImpl::is_present(matches, "verbose"),
        })
    }
}

impl Arguments {
    pub fn new() -> Result<Self> {
        let matcher = ArgMatcher::get_matched_args();
        Self::parse(&matcher)
            .and_then(Self::normalize_path)
            .and_then(Self::check)
    }

    fn normalize_path(mut self) -> Result<Self> {
        for source_file in &mut self.source {
            *source_file = fs::normalize(&source_file)?;
        }
        for debuginfo_file in &mut self.debuginfo {
            *debuginfo_file = fs::normalize(&debuginfo_file)?;
        }
        for patch_file in &mut self.patch {
            *patch_file = fs::normalize(&patch_file)?;
        }
        self.build_root = fs::normalize(&self.build_root)?;
        self.output = fs::normalize(&self.output)?;

        Ok(self)
    }

    fn check(self) -> Result<Self> {
        for source_file in &self.source {
            ensure!(
                source_file.is_file(),
                format!("Cannot find file \"{}\"", source_file.display())
            );
        }
        for debuginfo_file in &self.debuginfo {
            ensure!(
                debuginfo_file.is_file(),
                format!("Cannot find file \"{}\"", debuginfo_file.display())
            );
        }
        for patch_file in &self.patch {
            ensure!(
                patch_file.is_file(),
                format!("Cannot find file \"{}\"", patch_file.display())
            );
        }
        if self.patch_arch.as_str() != os::cpu::arch() {
            bail!("Cross compilation is unsupported");
        }

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
