use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct RestoreCommandExecutor;

impl CommandExecutor for RestoreCommandExecutor {
    fn invoke(&self, _args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        for (patch_name, status) in patch_manager.read_saved_patch_status()? {
            patch_manager.restore_all_patch_status(&patch_name, status)?;
        }

        Ok(0)
    }
}
