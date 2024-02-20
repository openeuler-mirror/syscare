use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::ArgMatches;
use log::LevelFilter;

use syscare_common::util::fs;

mod matcher;
mod parser;

use matcher::ArgMatcher;
use parser::{ArgParser, ArgParserImpl, Parser};

#[derive(Debug, Clone)]
pub struct Arguments {
    /// Run as a daemon
    pub daemon: bool,

    /// Daemon config directory
    pub config_dir: PathBuf,

    /// Daemon working directory
    pub work_dir: PathBuf,

    /// Daemon logging directory
    pub log_dir: PathBuf,

    /// Set the logging level ("trace"|"debug"|"info"|"warn"|"error")
    pub log_level: LevelFilter,
}

impl Parser<'_> for Arguments {
    fn parse(matches: &ArgMatches<'_>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            daemon: ArgParserImpl::is_present(matches, "daemon"),
            config_dir: ArgParserImpl::parse_arg(matches, "config_dir")?,
            work_dir: ArgParserImpl::parse_arg(matches, "work_dir")?,
            log_dir: ArgParserImpl::parse_arg(matches, "log_dir")?,
            log_level: ArgParserImpl::parse_arg(matches, "log_level")?,
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
        self.config_dir = fs::normalize(&self.config_dir)?;
        self.work_dir = fs::normalize(self.work_dir)?;
        self.log_dir = fs::normalize(&self.log_dir)?;

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
