use clap::Parser;
use log::{LevelFilter, debug};
use common::util::sys;

use crate::cmd::*;

use super::logger::Logger;

const CLI_NAME:    &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT:   &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Debug)]
#[derive(Parser)]
#[command(bin_name=CLI_NAME, version=CLI_VERSION, about=CLI_ABOUT)]
#[command(disable_help_subcommand(true))]
pub struct SyscareCLI {
    #[command(subcommand)]
    cmd: Command,
    /// Provide more detailed info
    #[arg(short, long)]
    verbose: bool
}

impl SyscareCLI {
    fn cli_main(cmd: &Command) -> std::io::Result<i32> {
        let cmd_arguments;
        let cmd_executor;

        match cmd {
            Command::Build { args } => {
                cmd_executor  = Box::new(BuildCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::CommandLineArguments(args.to_owned());
            }
            Command::Info { identifier } => {
                cmd_executor  = Box::new(InfoCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Target { identifier } => {
                cmd_executor  = Box::new(TargetCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Status { identifier } => {
                cmd_executor  = Box::new(StatusCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::List => {
                cmd_executor  = Box::new(ListCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::None;
            },
            Command::Apply { identifier } => {
                cmd_executor  = Box::new(ApplyCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Remove { identifier } => {
                cmd_executor  = Box::new(RemoveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Active { identifier } => {
                cmd_executor  = Box::new(ActiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Deactive { identifier } => {
                cmd_executor  = Box::new(DeactiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments(identifier.to_owned())
            },
            Command::Save => {
                cmd_executor  = Box::new(SaveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::None;
            },
            Command::Restore => {
                cmd_executor  = Box::new(RestoreCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::None;
            },
            Command::Reboot { target, force } => {
                cmd_executor  = Box::new(RebootCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::RebootArguments(target.to_owned(), force.to_owned());
            },
        };

        debug!("Command {:?}", cmd);
        let exit_code = cmd_executor.invoke(&cmd_arguments)?;
        debug!("Command {:?} done", cmd);

        Ok(exit_code)
    }
}

impl SyscareCLI {
    pub fn name() -> &'static str {
        CLI_NAME
    }

    pub fn version() -> &'static str {
        CLI_VERSION
    }

    pub fn run() -> std::io::Result<i32> {
        let cli = Self::parse();

        Logger::initialize(match cli.verbose {
            false => LevelFilter::Info,
            true  => LevelFilter::Debug,
        });

        Self::cli_main(&cli.cmd)
    }

    pub fn check_root_permission() -> std::io::Result<()> {
        const ROOT_UID: u32 = 0;

        if sys::user_id() != ROOT_UID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "This command has to be run with superuser privileges (under the root user on most systems)."
            ));
        }

        Ok(())
    }
}
