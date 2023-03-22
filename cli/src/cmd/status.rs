use log::info;

use crate::patch::{PatchManager, PatchStatus};

use super::{CommandExecutor, CommandArguments};

pub struct StatusCommandExecutor;

impl CommandExecutor for StatusCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::PatchOperationArguments(identifier) = args {
            let patch_manger = PatchManager::new()?;
            let patch_status = patch_manger.get_patch_status(identifier).unwrap_or_default();
            info!("{}", patch_status);

            if patch_status == PatchStatus::Unknown {
                return Ok(-1);
            }
        }

        Ok(0)
    }
}
