use log::{info, debug};

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct StatusCommandExecutor;

impl CommandExecutor for StatusCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("Handle Command \"status {}\"", args[0]);

        info!("{}", patch_manager.get_patch_status(&args[0])?);

        debug!("Command \"status {}\" done", args[0]);
        Ok(0)
    }
}
