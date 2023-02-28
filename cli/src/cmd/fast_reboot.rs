use log::info;

use crate::boot::{BootManager, RebootOption};
use crate::util::fs;

use super::{CommandExecutor, CommandArguments};

pub struct FastRebootCommandExecutor;

impl CommandExecutor for FastRebootCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::RebootArguments(kernel, force) => {
                if !force {
                    info!("Syncing filesystem");
                    fs::sync();
                }

                info!("Preparing for reboot");
                BootManager::load_kernel(kernel)?;

                info!("Rebooting system");
                BootManager::reboot(match force {
                    false => RebootOption::Normal,
                    true  => RebootOption::Forced,
                })?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
