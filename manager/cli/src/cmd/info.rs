use log::info;

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct InfoCommandExecutor;

impl CommandExecutor for InfoCommandExecutor {
    fn invoke(&self, args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        let patch_info = patch_manager.get_patch_info(&args[0])?;

        info!("===============================");
        info!("{}", patch_info);
        info!("===============================");
        Ok(0)
    }
}
