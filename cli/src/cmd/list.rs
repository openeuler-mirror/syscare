use log::info;

use crate::patch::PatchManager;

use super::{CommandArguments, CommandExecutor};

pub struct ListCommandExecutor;

impl CommandExecutor for ListCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        let patch_manager = PatchManager::new()?;
        let info_list = patch_manager
            .get_patch_list()
            .iter()
            .map(|patch| {
                (
                    &patch.uuid,
                    patch.full_name(),
                    patch.status().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();

        if info_list.is_empty() {
            return Ok(0);
        }

        info!("{:<40} {:<40} {:<12}", "Uuid", "Name", "Status");
        for (uuid, name, status) in info_list {
            info!("{:<40} {:<40} {:<12}", uuid, name, status);
        }

        Ok(0)
    }
}
