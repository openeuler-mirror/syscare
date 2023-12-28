use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::ArgMatches;

mod matcher;
mod parser;
mod subcommand;

use matcher::ArgMatcher;
use parser::Parser;
pub use subcommand::SubCommand;

use parser::{ArgParser, ArgParserImpl};
use syscare_common::util::fs;

#[derive(Debug)]
pub struct Arguments {
    pub command: SubCommand,
    pub work_dir: PathBuf,
    pub verbose: bool,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        let matcher = ArgMatcher::get_matched_args();
        Self::parse(&matcher)
            .and_then(Self::normalize_pathes)
            .context("Failed to parse arguments")
    }

    fn normalize_pathes(mut self) -> Result<Self> {
        self.work_dir = fs::normalize(self.work_dir)?;
        Ok(self)
    }
}

impl Parser<'_> for Arguments {
    fn parse(matches: &ArgMatches<'_>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            command: SubCommand::parse(matches)?,
            work_dir: ArgParserImpl::parse_arg(matches, "work_dir")?,
            verbose: ArgParserImpl::parse_arg(matches, "verbose")?,
        })
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
