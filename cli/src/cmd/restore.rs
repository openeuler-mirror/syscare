use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct RestoreCommandExecutor;

impl CommandExecutor for RestoreCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        if let Err(e) = PatchManager::new()?.restore_all_patch_status() {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
        Ok(0)
    }
}
