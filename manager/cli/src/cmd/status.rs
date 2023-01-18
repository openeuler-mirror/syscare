use log::info;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct StatusCommandExecutor;

impl CommandExecutor for StatusCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        info!("{}", PatchManager::new()?.get_patch_status(&args[0])?);

        Ok(0)
    }
}
