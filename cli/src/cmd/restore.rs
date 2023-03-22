use common::os::signal;
use common::os::signal::{SIGINT, SIGTERM};

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct RestoreCommandExecutor;

impl CommandExecutor for RestoreCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        signal::block(&[SIGINT, SIGTERM])?;

        if let Err(e) = PatchManager::new()?.restore_all_patch_status() {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }

        Ok(0)
    }
}
