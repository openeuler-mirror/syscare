use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct InfoCommandExecutor;

impl CommandExecutor for InfoCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        match args {
            CommandArguments::PatchOperationArguments(identifier) => {
                PatchManager::new()?
                    .get_patch_info(&identifier)?
                    .print_log(log::Level::Info);

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
