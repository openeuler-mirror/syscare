use log::{info, debug};

use crate::patch::PatchManager;

use super::CommandExecutor;

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &[String]) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        debug!("handle command \"list\"");

        info!("{:<35} {:<35} {:<12}", "PackageName", "PatchName", "PatchStatus");
        for patch in patch_manager.get_patch_list() {
            info!("{:<35} {:<35} {:<12}",
                patch.get_target().get_simple_name(),
                patch.get_simple_name(),
                patch.get_status()
            );
        }

        debug!("command \"list\" done");
        Ok(0)
    }
}
