use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct TargetCommandExecutor;

impl CommandExecutor for TargetCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::PatchOperationArguments { identifier } = args {
            PatchManager::new()?.find_patch(&identifier)?.info().target.print_log(log::Level::Info);
        }

        Ok(0)
    }
}
