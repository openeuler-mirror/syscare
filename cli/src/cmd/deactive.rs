use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct DeactiveCommandExecutor;

impl CommandExecutor for DeactiveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                PatchManager::new()?.deactive_patch(&patch_name)?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
