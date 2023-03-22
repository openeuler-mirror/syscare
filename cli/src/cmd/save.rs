use common::os::signal;
use common::os::signal::{SIGINT, SIGTERM};

use crate::cli::SyscareCLI;
use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct SaveCommandExecutor;

impl CommandExecutor for SaveCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        SyscareCLI::check_root_permission()?;
        signal::block(&[SIGINT, SIGTERM])?;

        PatchManager::new()?.save_all_patch_status()?;

        Ok(0)
    }
}
