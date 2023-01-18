use std::sync::Once;

use clap::Parser;
use log::{LevelFilter, debug};

use crate::log::Logger;
use crate::util::sys;
use crate::cmd::*;

pub const CLI_NAME: &str = env!("CARGO_PKG_NAME");
const CLI_AUTHOR:   &str = env!("CARGO_PKG_AUTHORS");
const CLI_VERSION:  &str = env!("CARGO_PKG_VERSION");
const CLI_ABOUT:    &str = env!("CARGO_PKG_DESCRIPTION");

const ROOT_UID: u32 = 0;

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
    pub fn new() -> Self {
        Self::parse()
    }

    fn initialize(&self) {
        static INITIALIZE: Once = Once::new();

        INITIALIZE.call_once(|| {
            let log_level = match self.verbose {
                false => LevelFilter::Info,
                true  => LevelFilter::Debug,
            };
            Logger::init_logger(log_level);
        });
    }

    fn check_root_permission(&self) -> std::io::Result<()> {
        if sys::get_uid() != ROOT_UID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "this command has to be run with superuser privileges (under the root user on most systems)."
            ));
        }

        Ok(())
    }

    pub fn run(&self) -> std::io::Result<i32> {
        self.initialize();

        let cmd = &self.cmd;
        let cmd_args;
        let cmd_executor;

        match cmd {
            Command::Build { args } => {
                cmd_executor = Box::new(BuildCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = args.to_owned();
            }
            Command::Info { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(InfoCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Status { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(StatusCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::List => {
                self.check_root_permission()?;
                cmd_executor = Box::new(ListCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![];
            },
            Command::Apply { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(ApplyCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Remove { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(RemoveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Active { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(ActiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Deactive { patch_name } => {
                self.check_root_permission()?;
                cmd_executor = Box::new(DeactiveCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![patch_name.to_owned()];
            },
            Command::Restore => {
                self.check_root_permission()?;
                cmd_executor = Box::new(RestoreCommandExecutor {}) as Box<dyn CommandExecutor>;
                cmd_args = vec![];
            },
        };

        debug!("{:?}", cmd);
        let exit_code = cmd_executor.invoke(&cmd_args)?;
        debug!("{:?} finished", cmd);

        Ok(exit_code)
    }
}
