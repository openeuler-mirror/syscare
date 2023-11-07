use anyhow::{bail, Result};
use clap::ArgMatches;

use super::parser::{ArgParser, ArgParserImpl, Parser};

#[derive(Debug, Clone)]
pub enum SubCommand {
    Build {
        args: Vec<String>,
    },
    Info {
        identifiers: Vec<String>,
    },
    Target {
        identifiers: Vec<String>,
    },
    Status {
        identifiers: Vec<String>,
    },
    List,
    Check {
        identifiers: Vec<String>,
    },
    Apply {
        identifiers: Vec<String>,
        force: bool,
    },
    Remove {
        identifiers: Vec<String>,
    },
    Active {
        identifiers: Vec<String>,
    },
    Deactive {
        identifiers: Vec<String>,
    },
    Accept {
        identifiers: Vec<String>,
    },
    Save,
    Restore {
        accepted: bool,
    },
    Reboot {
        target: Option<String>,
        force: bool,
    },
}

impl Parser<'_> for SubCommand {
    fn parse(matches: &ArgMatches<'_>) -> Result<Self>
    where
        Self: Sized,
    {
        let subcommand = match matches.subcommand() {
            ("build", Some(cmd_matches)) => Self::Build {
                args: ArgParserImpl::parse_args(cmd_matches, "args")?,
            },
            ("info", Some(cmd_matches)) => Self::Info {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("target", Some(cmd_matches)) => Self::Target {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("status", Some(cmd_matches)) => Self::Status {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("list", Some(_)) => Self::List,
            ("check", Some(cmd_matches)) => Self::Check {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("apply", Some(cmd_matches)) => Self::Apply {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
                force: ArgParserImpl::is_present(cmd_matches, "force"),
            },
            ("remove", Some(cmd_matches)) => Self::Remove {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("active", Some(cmd_matches)) => Self::Active {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("deactive", Some(cmd_matches)) => Self::Deactive {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("accept", Some(cmd_matches)) => Self::Accept {
                identifiers: ArgParserImpl::parse_args(cmd_matches, "identifier")?,
            },
            ("save", Some(_)) => Self::Save,
            ("restore", Some(cmd_matches)) => Self::Restore {
                accepted: ArgParserImpl::is_present(cmd_matches, "accepted"),
            },
            (cmd_name, _) => bail!("Subcommand \"{}\" is invalid", cmd_name),
        };

        Ok(subcommand)
    }
}
