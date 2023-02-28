use log::info;

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct StatusCommandExecutor;

impl CommandExecutor for StatusCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                info!("{}", PatchManager::new()?.get_patch_status(&patch_name)?);

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
