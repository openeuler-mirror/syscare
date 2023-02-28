use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                let mut patch_manager = PatchManager::new()?;
                patch_manager.deactive_patch(&patch_name)?;
                patch_manager.save_all_patch_status()?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
