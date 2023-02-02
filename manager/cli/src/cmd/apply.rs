use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ApplyCommandExecutor;

impl CommandExecutor for ApplyCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        debug!("handle command \"apply {}\"", args[0]);

        patch_manager.apply_patch(&args[0])?;
        patch_manager.save_all_patch_status()?;

        debug!("command \"apply {}\" done", args[0]);
        Ok(0)
    }
}
