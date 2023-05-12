use common::os;

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        os::signal::block(&[os::signal::SIGINT, os::signal::SIGTERM])?;

        if let CommandArguments::PatchOperationArguments(identifier) = args {
            PatchManager::new()?.find_patch(identifier)?.deactive()?;
        }

        Ok(0)
    }
}
