use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ActiveCommandExecutor;

impl CommandExecutor for ActiveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        debug!("handle command \"active {}\"", args[0]);

        patch_manager.active_patch(&args[0])?;
        patch_manager.save_all_patch_status()?;

        debug!("command \"active {}\" done", args[0]);
        Ok(0)
    }
}
