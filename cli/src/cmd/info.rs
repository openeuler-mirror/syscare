use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct InfoCommandExecutor;

impl CommandExecutor for InfoCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("Handle Command \"info {}\"", args[0]);

        patch_manager.get_patch_info(&args[0])?
                     .print_log(log::Level::Info);

        debug!("Command \"info {}\" done", args[0]);
        Ok(0)
    }
}
