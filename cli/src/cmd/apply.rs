use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct ApplyCommandExecutor;

impl CommandExecutor for ApplyCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                let mut patch_manager = PatchManager::new()?;
                patch_manager.apply_patch(&patch_name)?;
                patch_manager.save_all_patch_status()?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
