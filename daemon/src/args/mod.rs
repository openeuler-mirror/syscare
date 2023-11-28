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

    /// Path for daemon pid file
    pub pid_file: PathBuf,

    /// Path for daemon unix socket
    pub socket_file: PathBuf,

    /// Daemon working directory
    pub work_dir: PathBuf,

    /// Daemon data directory
    pub data_dir: PathBuf,

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
            pid_file: ArgParserImpl::parse_arg(matches, "pid_file")?,
            socket_file: ArgParserImpl::parse_arg(matches, "socket_file")?,
            work_dir: ArgParserImpl::parse_arg(matches, "work_dir")?,
            data_dir: ArgParserImpl::parse_arg(matches, "data_dir")?,
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
        self.pid_file = fs::normalize(&self.pid_file)?;
        self.socket_file = fs::normalize(&self.socket_file)?;
        self.work_dir = fs::normalize(self.work_dir)?;
        self.data_dir = fs::normalize(&self.data_dir)?;
        self.log_dir = fs::normalize(&self.log_dir)?;

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
