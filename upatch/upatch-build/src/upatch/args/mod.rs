use std::{ffi::OsString, path::PathBuf};

use anyhow::bail;
use clap::ArgMatches;

use crate::tool::*;

mod matcher;
mod parser;

use matcher::ArgMatcher;
use parser::{ArgParser, ArgParserImpl, Parser};

use super::Result;

#[derive(Debug, Clone)]
pub struct Arguments {
    /// Specify patch name
    pub name: OsString,

    /// Specify work directory
    pub work_dir: PathBuf,

    /// Specify source directory
    pub source_dir: PathBuf,

    /// Specify build source command
    pub build_source_cmd: String,

    /// Specify build patched command
    pub build_patch_cmd: String,

    /// Specify debug info list
    pub debuginfo: Vec<PathBuf>,

    /// Specify the directory of searching elf
    pub elf_dir: Option<PathBuf>,

    /// Specify elf's relative path relate to elf-dir or absolute path list
    pub elf_path: Vec<PathBuf>,

    /// Specify compiler
    pub compiler: Vec<PathBuf>,

    /// Specify output directory
    pub output_dir: PathBuf,

    /// Skip compiler version check (not recommended)
    pub skip_compiler_check: bool,

    /// Provide more detailed info
    pub verbose: bool,

    /// Patch file(s)
    pub patches: Vec<PathBuf>,
}

impl Parser<'_> for Arguments {
    fn parse(matches: &ArgMatches<'_>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            name: match ArgParserImpl::is_present(matches, "name") {
                false => OsString::default(),
                true => ArgParserImpl::parse_arg(matches, "name")?,
            },
            work_dir: ArgParserImpl::parse_arg(matches, "work_dir")?,
            source_dir: ArgParserImpl::parse_arg(matches, "source_dir")?,
            build_source_cmd: ArgParserImpl::parse_arg(matches, "build_source_cmd")?,
            build_patch_cmd: ArgParserImpl::parse_arg(matches, "build_patch_cmd")?,
            debuginfo: ArgParserImpl::parse_args(matches, "debuginfo")?,
            elf_dir: match ArgParserImpl::is_present(matches, "elf_dir") {
                false => None,
                true => Some(ArgParserImpl::parse_arg(matches, "elf_dir")?),
            },
            elf_path: ArgParserImpl::parse_args(matches, "elf_path")?,
            compiler: ArgParserImpl::parse_args(matches, "compiler")?,
            output_dir: ArgParserImpl::parse_arg(matches, "output_dir")?,
            skip_compiler_check: ArgParserImpl::is_present(matches, "skip_compiler_check"),
            verbose: ArgParserImpl::is_present(matches, "verbose"),
            patches: ArgParserImpl::parse_args(matches, "patches")?,
        })
    }
}

impl Arguments {
    pub fn new() -> Result<Self> {
        let matcher = ArgMatcher::get_matched_args();
        let args = Self::parse(&matcher)
            .and_then(Self::check)
            .map_err(|e| super::Error::Mod(e.to_string()))?;

        Ok(args)
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

        for patch in &mut self.patches {
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