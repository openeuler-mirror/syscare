use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        let patch_list    = patch_manager.get_patch_list()
            .into_iter()
            .map(|patch| (
                patch.target.short_name(),
                patch.short_name(),
                patch.status().unwrap_or_default(),
            )).collect::<Vec<_>>();

        if patch_list.len() == 0 {
            return Ok(0)
        }

        info!("{:<35} {:<25} {:<12}", "Target", "Name", "Status");
        for (target_name, patch_name, patch_status) in patch_list {
            info!("{:<35} {:<25} {:<12}", target_name, patch_name, patch_status);
        }

        Ok(0)
    }
}
