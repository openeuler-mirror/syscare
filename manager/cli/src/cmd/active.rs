use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ActiveCommandExecutor;

impl CommandExecutor for ActiveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        PatchManager::new()?.active_patch(&args[0])?;

        Ok(0)
    }
}
