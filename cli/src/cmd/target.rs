use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct TargetCommandExecutor;

impl CommandExecutor for TargetCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::PatchOperationArguments(identifier) = args {
            PatchManager::new()?
                .get_patch_target(&identifier)?
                .print_log(log::Level::Info);
        }

        Ok(0)
    }
}
