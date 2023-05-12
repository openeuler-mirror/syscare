use common::os;

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct RemoveCommandExecutor;

impl CommandExecutor for RemoveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        os::signal::block(&[os::signal::SIGINT, os::signal::SIGTERM])?;

        if let CommandArguments::PatchOperationArguments { identifier } = args {
            PatchManager::new()?.find_patch(identifier)?.remove()?;
        }

        Ok(0)
    }
}
