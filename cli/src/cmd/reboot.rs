use log::debug;
use common::util::fs;

use crate::boot::{BootManager, RebootOption};

use super::{CommandExecutor, CommandArguments};

pub struct RebootCommandExecutor;

impl CommandExecutor for RebootCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::RebootArguments(target, force) => {
                if !force {
                    debug!("Syncing filesystem");
                    fs::sync();
                }

                debug!("Preparing for reboot");
                BootManager::load_kernel(target.as_ref())?;


                BootManager::reboot(match force {
                    false => {
                        debug!("Rebooting system");
                        RebootOption::Normal
                    },
                    true => {
                        debug!("Force rebooting system");
                        RebootOption::Forced
                    }
                })?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
