use common::os::signal;
use common::os::signal::{SIGINT, SIGTERM};

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct ActiveCommandExecutor;

impl CommandExecutor for ActiveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        signal::block(&[SIGINT, SIGTERM])?;

        if let CommandArguments::PatchOperationArguments(identifier) = args {
            PatchManager::new()?.active_patch(&identifier)?;
        }

        Ok(0)
    }
}
