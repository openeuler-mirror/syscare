use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        let patch_info_iter = patch_manager.get_patch_list()
            .into_iter()
            .map(|patch| (
                patch.target.short_name(),
                patch.short_name(),
                patch.status().unwrap_or_default(),
            ));

        info!("{:<35} {:<35} {:<12}", "PackageName", "PatchName", "PatchStatus");
        for (target_name, patch_name, patch_status) in patch_info_iter {
            info!("{:<35} {:<35} {:<12}", target_name, patch_name, patch_status);
        }

        Ok(0)
    }
}
