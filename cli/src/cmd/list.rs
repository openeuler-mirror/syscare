use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        let info_list     = patch_manager.get_patch_list()
            .into_iter()
            .map(|patch| (
                &patch.uuid,
                format!("{}/{}", patch.target.short_name(), patch.short_name()),
                patch.status().unwrap_or_default(),
            )).collect::<Vec<_>>();

        if info_list.len() == 0 {
            return Ok(0)
        }

        info!("{:<40} {:<40} {:<12}", "Uuid", "Name", "Status");
        for (uuid, name, status) in info_list {
            info!("{:<40} {:<40} {:<12}", uuid, name, status);
        }

        Ok(0)
    }
}
