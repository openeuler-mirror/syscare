use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct TargetCommandExecutor;

impl CommandExecutor for TargetCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(patch_name) => {
                PatchManager::new()?
                    .get_patch_target(&patch_name)?
                    .print_log(log::Level::Info);

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
