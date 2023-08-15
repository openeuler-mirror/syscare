use std::path::PathBuf;

use clap::{Parser, Subcommand};

use super::{CLI_NAME, CLI_VERSION};

const DEFAULT_SOCKET_FILE: &str = "/var/run/syscare.sock";

#[derive(Parser, Debug)]
#[clap(bin_name=CLI_NAME, version=CLI_VERSION)]
pub struct CliArguments {
    /// Command name
    #[clap(subcommand)]
    pub command: CliCommand,

    /// Path for daemon unix socket
    #[clap(long, default_value=DEFAULT_SOCKET_FILE)]
    pub socket_file: PathBuf,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CliCommand {
    /// Build a new patch
    #[clap(
        disable_help_flag(true),
        subcommand_precedence_over_arg(true),
        allow_hyphen_values(true)
    )]
    Build { args: Vec<String> },
    /// Show patch detail info
    Info {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Show patch target info
    Target {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Show patch status
    Status {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// List all installed patches
    List,
    /// Apply a patch
    Apply {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Remove a patch
    Remove {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Activate a patch
    Active {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Deactive a patch
    Deactive {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Accept a patch
    Accept {
        /// Patch identifier, typically would be "<TARGET_NAME>/<PATCH_NAME>"
        identifier: String,
    },
    /// Save all patch status
    Save,
    /// Restore all patch status
    Restore {
        /// Only restore ACCEPTED patches
        #[clap(long)]
        accepted: bool,
    },
    /// Reboot the system
    Reboot {
        /// Target kernel name
        #[clap(short, long)]
        target: Option<String>,
        #[clap(short, long)]
        /// Skip all checks, force reboot
        force: bool,
    },
}

impl std::fmt::Display for CliArguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
