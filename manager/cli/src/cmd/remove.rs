use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct RemoveCommandExecutor;

impl CommandExecutor for RemoveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        PatchManager::new()?.remove_patch(&args[0])?;

        Ok(0)
    }
}
