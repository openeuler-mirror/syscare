use log::{info, debug};

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct InfoCommandExecutor;

impl CommandExecutor for InfoCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("handle command \"info {}\"", args[0]);

        info!("{}", patch_manager.get_patch_info(&args[0])?);

        debug!("command \"info {}\" done", args[0]);
        Ok(0)
    }
}
