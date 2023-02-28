use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct RestoreCommandExecutor;

impl CommandExecutor for RestoreCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        PatchManager::new()?.restore_all_patch_status()?;

        Ok(0)
    }
}
