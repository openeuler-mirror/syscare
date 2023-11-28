use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::{AppSettings, ColorChoice, Parser, Subcommand};

use syscare_common::util::fs;

use super::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_SOCKET_FILE: &str = "/var/run/syscared.sock";

#[derive(Parser, Debug)]
#[clap(
    bin_name = CLI_NAME,
    version = CLI_VERSION,
    about = CLI_ABOUT,
    arg_required_else_help(true),
    color(ColorChoice::Never),
    disable_help_subcommand(true),
    global_setting(AppSettings::DeriveDisplayOrder),
    term_width(120),
)]
pub struct Arguments {
    /// Command name
    #[clap(subcommand)]
    pub command: SubCommand,

    /// Path for daemon unix socket
    #[clap(short, long, default_value=DEFAULT_SOCKET_FILE)]
    pub socket_file: PathBuf,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SubCommand {
    /// Build a patch
    #[clap(
        disable_help_flag(true),
        subcommand_precedence_over_arg(true),
        allow_hyphen_values(true)
    )]
    Build { args: Vec<String> },
    /// Show patch info
    Info {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Show patch target
    Target {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Show patch status
    Status {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// List all patches
    List,
    /// Check a patch
    Check {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Apply a patch
    Apply {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
        /// Force to apply a patch
        #[clap(short, long)]
        force: bool,
    },
    /// Remove a patch
    Remove {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Active a patch
    Active {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Deactive a patch
    Deactive {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Accept a patch
    Accept {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Save all patch status
    Save,
    /// Restore all patch status
    Restore {
        /// Only restore ACCEPTED patches
        #[clap(long)]
        accepted: bool,
    },
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize_path().and_then(Self::check)
    }

    fn normalize_path(mut self) -> Result<Self> {
        self.socket_file = fs::normalize(&self.socket_file)?;

        Ok(self)
    }

    fn check(self) -> Result<Self> {
        let socket_file = &self.socket_file;
        ensure!(
            socket_file.exists() || !socket_file.is_dir(),
            format!("Cannot find file \"{}\"", socket_file.display())
        );

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
