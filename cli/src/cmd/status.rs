use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct StatusCommandExecutor;

impl CommandExecutor for StatusCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::PatchOperationArguments { identifier } = args {
            info!("{}", PatchManager::new()?.find_patch(identifier)?.status()?);
        }

        Ok(0)
    }
}
