use log::{info, debug};

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct TargetCommandExecutor;

impl CommandExecutor for TargetCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("handle command \"info {}\"", args[0]);

        info!("{}", patch_manager.get_patch_target(&args[0])?);

        debug!("command \"target {}\" done", args[0]);
        Ok(0)
    }
}
