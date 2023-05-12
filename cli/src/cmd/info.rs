use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct InfoCommandExecutor;

impl CommandExecutor for InfoCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        if let CommandArguments::PatchOperationArguments { identifier } = args {
            PatchManager::new()?.find_patch(identifier)?.info().print_log(log::Level::Info);
        }

        Ok(0)
    }
}
