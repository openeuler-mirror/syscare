use common::os;

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct RestoreCommandExecutor;

impl CommandExecutor for RestoreCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        os::signal::block(&[os::signal::SIGINT, os::signal::SIGTERM])?;

        if let CommandArguments::PatchRestoreArguments { accepted_only } = args {
            if let Err(e) = PatchManager::new()?.restore_all_patch_status(*accepted_only) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(e);
                }
            }
        }

        Ok(0)
    }
}
