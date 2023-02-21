use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct RemoveCommandExecutor;

impl CommandExecutor for RemoveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        debug!("Handle Command \"remove {}\"", args[0]);

        patch_manager.remove_patch(&args[0])?;
        patch_manager.save_all_patch_status()?;

        debug!("Command \"remove {}\" done", args[0]);
        Ok(0)
    }
}
