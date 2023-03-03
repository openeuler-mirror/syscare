use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ActiveCommandExecutor;

impl CommandExecutor for ActiveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                PatchManager::new()?.active_patch(&patch_name)?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
