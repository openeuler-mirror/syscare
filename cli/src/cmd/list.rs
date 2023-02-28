use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;

        info!("{:<35} {:<35} {:<12}", "PackageName", "PatchName", "PatchStatus");
        for patch in patch_manager.get_patch_list() {
            info!("{:<35} {:<35} {:<12}",
                patch.target.short_name(),
                patch.short_name(),
                patch.status
            );
        }

        Ok(0)
    }
}
