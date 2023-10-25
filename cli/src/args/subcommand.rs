use anyhow::{bail, Result};
use clap::ArgMatches;

use super::parser::{ArgParser, ArgParserImpl, Parser};

#[derive(Debug, Clone)]
pub enum SubCommand {
    Build { args: Vec<String> },
    Info { identifier: String },
    Target { identifier: String },
    Status { identifier: String },
    List,
    Check { identifier: String },
    Apply { identifier: String, force: bool },
    Remove { identifier: String },
    Active { identifier: String },
    Deactive { identifier: String },
    Accept { identifier: String },
    Save,
    Restore { accepted: bool },
    Reboot { target: Option<String>, force: bool },
}

impl Parser<'_> for SubCommand {
    fn parse(matches: &ArgMatches<'_>) -> Result<Self>
    where
        Self: Sized,
    {
        let subcommand = match matches.subcommand() {
            ("build", Some(cmd_matches)) => Self::Build {
                args: match ArgParserImpl::is_present(cmd_matches, "args") {
                    false => vec![],
                    true => ArgParserImpl::parse_args(cmd_matches, "args")?,
                },
            },
            ("info", Some(cmd_matches)) => Self::Info {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("target", Some(cmd_matches)) => Self::Target {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("status", Some(cmd_matches)) => Self::Status {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("list", Some(_)) => Self::List,
            ("check", Some(cmd_matches)) => Self::Check {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("apply", Some(cmd_matches)) => Self::Apply {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
                force: ArgParserImpl::is_present(cmd_matches, "force"),
            },
            ("remove", Some(cmd_matches)) => Self::Remove {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("active", Some(cmd_matches)) => Self::Active {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("deactive", Some(cmd_matches)) => Self::Deactive {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
            },
            ("accept", Some(cmd_matches)) => Self::Accept {
                identifier: ArgParserImpl::parse_arg(cmd_matches, "identifier")?,
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
