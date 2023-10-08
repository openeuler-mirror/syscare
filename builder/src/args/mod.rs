use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::ArgMatches;

use syscare_common::util::fs;

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

    /// Source package
    pub source: Vec<PathBuf>,

    /// Debuginfo package(s)
    pub debuginfo: Vec<PathBuf>,

    /// Patch file(s)
    pub patch: Vec<PathBuf>,

    /// Working directory
    pub workdir: PathBuf,

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
            source: ArgParserImpl::parse_args(matches, "source")?,
            debuginfo: ArgParserImpl::parse_args(matches, "debuginfo")?,
            patch: ArgParserImpl::parse_args(matches, "patch")?,
            workdir: ArgParserImpl::parse_arg(matches, "workdir")?,
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
            .and_then(Self::normalize_pathes)
            .context("Failed to parse arguments")
    }

    fn normalize_pathes(mut self) -> Result<Self> {
        for source in &mut self.source {
            *source = fs::normalize(&source)?;
        }
        for debuginfo in &mut self.debuginfo {
            *debuginfo = fs::normalize(&debuginfo)?;
        }
        self.workdir = fs::normalize(&self.workdir)?;
        self.output = fs::normalize(&self.output)?;
        for patches in &mut self.patch {
            *patches = fs::normalize(&patches)?;
        }
        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
