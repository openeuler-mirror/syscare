use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ApplyCommandExecutor;

impl CommandExecutor for ApplyCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                PatchManager::new()?.apply_patch(&patch_name)?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
