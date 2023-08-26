use anyhow::{ensure, Result};
use syscare_common::os;

use super::args::CliCommand;

pub mod build;
pub mod patch;
pub mod reboot;

pub trait CommandExecutor {
    fn invoke(&self, command: &CliCommand) -> Result<()>;

    fn check_root_permission(&self) -> Result<()> {
        const ROOT_UID: u32 = 0;

        ensure!(
            os::user::id() == ROOT_UID,
            "This command has to be run with superuser privileges (under the root user on most systems)."
        );

        Ok(())
    }
}
