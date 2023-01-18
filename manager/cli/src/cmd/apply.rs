use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ApplyCommandExecutor;

impl CommandExecutor for ApplyCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        PatchManager::new()?.apply_patch(&args[0])?;

        Ok(0)
    }
}
