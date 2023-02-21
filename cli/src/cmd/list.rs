use log::{info, debug};

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("Handle Command \"list\"");

        info!("{:<35} {:<35} {:<12}", "PackageName", "PatchName", "PatchStatus");
        for patch in patch_manager.get_patch_list() {
            info!("{:<35} {:<35} {:<12}",
                patch.target.short_name(),
                patch.short_name(),
                patch.status
            );
        }

        debug!("Command \"list\" done");
        Ok(0)
    }
}
