use clap::Parser;
use common::os;
use log::{debug, LevelFilter};

use crate::cmd::*;

use super::logger::Logger;

const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const CLI_UMASK: u32 = 0o022;

#[derive(Debug, Parser)]
#[command(bin_name=CLI_NAME, version=CLI_VERSION, about=CLI_ABOUT)]
#[command(disable_help_subcommand(true))]
pub struct SyscareCLI {
    #[command(subcommand)]
    cmd: Command,
    /// Provide more detailed info
    #[arg(short, long)]
    verbose: bool,
}

impl SyscareCLI {
    fn initialize(&self) -> std::io::Result<()> {
        os::umask::set_umask(CLI_UMASK);
        Logger::initialize(match self.verbose {
            false => LevelFilter::Info,
            true => LevelFilter::Debug,
        });

        Ok(())
    }

    fn cli_main(self) -> std::io::Result<i32> {
        self.initialize()?;

        let cmd_str = self.cmd.to_string();
        let cmd_arguments;
        let cmd_executor;

        match self.cmd {
            Command::Build { args } => {
                cmd_executor = Box::new(BuildCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::CommandLineArguments { args };
            }
            Command::Info { identifier } => {
                cmd_executor = Box::new(InfoCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Target { identifier } => {
                cmd_executor = Box::new(TargetCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Status { identifier } => {
                cmd_executor = Box::new(StatusCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::List => {
                cmd_executor = Box::new(ListCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::None;
            }
            Command::Apply { identifier } => {
                cmd_executor = Box::new(ApplyCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Remove { identifier } => {
                cmd_executor = Box::new(RemoveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Active { identifier } => {
                cmd_executor = Box::new(ActiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Deactive { identifier } => {
                cmd_executor = Box::new(DeactiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Accept { identifier } => {
                cmd_executor = Box::new(AcceptCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchOperationArguments { identifier };
            }
            Command::Save => {
                cmd_executor = Box::new(SaveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::None;
            }
            Command::Restore { accepted } => {
                cmd_executor = Box::new(RestoreCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::PatchRestoreArguments {
                    accepted_only: accepted,
                };
            }
            Command::Reboot { target, force } => {
                cmd_executor = Box::new(RebootCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_arguments = CommandArguments::RebootArguments { target, force };
            }
        };

        debug!("Command {}", cmd_str);
        let exit_code = cmd_executor.invoke(&cmd_arguments)?;
        debug!("Command {} done", cmd_str);

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
        Self::parse().cli_main()
    }

    pub fn check_root_permission() -> std::io::Result<()> {
        const ROOT_UID: u32 = 0;

        if os::user::id() != ROOT_UID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "This command has to be run with superuser privileges (under the root user on most systems)."
            ));
        }

        Ok(())
    }
}
