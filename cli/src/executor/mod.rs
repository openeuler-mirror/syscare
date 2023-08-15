use anyhow::Result;

use super::args::CliCommand;

pub mod build;
pub mod patch;
pub mod reboot;

pub trait CommandExecutor {
    fn invoke(&self, command: &CliCommand) -> Result<()>;
}
