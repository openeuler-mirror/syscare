use log::debug;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let mut patch_manager = PatchManager::new()?;
        debug!("handle command \"deactive {}\"", args[0]);

        patch_manager.deactive_patch(&args[0])?;

        debug!("command \"deactive {}\" done", args[0]);
        Ok(0)
    }
}
