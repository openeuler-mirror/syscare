use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct TargetCommandExecutor;

impl CommandExecutor for TargetCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("handle command \"info {}\"", args[0]);

        patch_manager.get_patch_target(&args[0])?
                     .print_log(log::Level::Info);

        debug!("command \"target {}\" done", args[0]);
        Ok(0)
    }
}
