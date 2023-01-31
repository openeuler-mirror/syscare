use clap::Parser;
use log::LevelFilter;

use crate::log::Logger;
use crate::util::sys;
use crate::cmd::*;

const CLI_NAME:    &str = env!("CARGO_PKG_NAME");
const CLI_AUTHOR:  &str = env!("CARGO_PKG_AUTHORS");
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT:   &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Debug)]
#[derive(Parser)]
#[command(bin_name=CLI_NAME, author=CLI_AUTHOR, version=CLI_VERSION, about=CLI_ABOUT)]
#[command(disable_help_subcommand(true))]
pub struct SyscareCLI {
    #[command(subcommand)]
    cmd: Command,
    /// Provide more detailed info
    #[arg(short, long)]
    verbose: bool
}

impl SyscareCLI {
    fn check_root_permission() -> std::io::Result<()> {
        const ROOT_UID: u32 = 0;

        if sys::get_uid() != ROOT_UID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "this command has to be run with superuser privileges (under the root user on most systems)."
            ));
        }

        Ok(())
    }

    fn cli_main(cmd: &Command) -> std::io::Result<i32> {
        let cmd_args;
        let cmd_executor;

        match cmd {
            Command::Build { args } => {
                cmd_executor = Box::new(BuildCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = args.to_owned();
            }
            Command::Info { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(InfoCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Status { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(StatusCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::List => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(ListCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![];
            },
            Command::Apply { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(ApplyCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Remove { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(RemoveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Active { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(ActiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Deactive { patch_name } => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(DeactiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Restore => {
                Self::check_root_permission()?;
                cmd_executor = Box::new(RestoreCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![];
            },
        };

        Ok(cmd_executor.invoke(&cmd_args)?)
    }
}

impl SyscareCLI {
    pub fn name() -> &'static str {
        CLI_NAME
    }

    pub fn run() -> std::io::Result<i32> {
        let cli = Self::parse();

        Logger::initialize(match cli.verbose {
            false => LevelFilter::Info,
            true  => LevelFilter::Debug,
        });

        Self::cli_main(&cli.cmd)
    }
}
