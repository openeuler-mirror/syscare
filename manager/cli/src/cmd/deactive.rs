use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        debug!("Handle Command \"deactive {}\"", args[0]);

        patch_manager.deactive_patch(&args[0])?;
        patch_manager.save_all_patch_status()?;

        debug!("Command \"deactive {}\" done", args[0]);
        Ok(0)
    }
}
