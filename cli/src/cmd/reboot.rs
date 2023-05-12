use log::debug;
use common::util::fs;

use crate::cli::SyscareCLI;
use crate::boot::{BootManager, RebootOption};

use super::{CommandArguments, CommandExecutor};

pub struct RebootCommandExecutor;

impl CommandExecutor for RebootCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;

        if let CommandArguments::RebootArguments { target, force } = args {
            if !force {
                debug!("Syncing filesystem");
                fs::sync();
            }

            debug!("Preparing kernel images");
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
        }

        Ok(0)
    }
}
