use common::os::signal;
use common::os::signal::{SIGINT, SIGTERM};

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct RemoveCommandExecutor;

impl CommandExecutor for RemoveCommandExecutor {
    fn invoke(&self, args: &CommandArguments) -> std::io::Result<i32> {
        signal::block(&[SIGINT, SIGTERM])?;

        match args {
            CommandArguments::PatchOperationArguments(identifier) => {
                PatchManager::new()?.remove_patch(&identifier)?;

                Ok(0)
            },
            _ => unreachable!(),
        }
    }
}
