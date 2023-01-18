use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        PatchManager::new()?.deactive_patch(&args[0])?;

        Ok(0)
    }
}
