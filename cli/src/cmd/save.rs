use common::os::signal;
use common::os::signal::{SIGINT, SIGTERM};

use crate::patch::PatchManager;

use super::{CommandExecutor, CommandArguments};

pub struct SaveCommandExecutor;

impl CommandExecutor for SaveCommandExecutor {
    fn invoke(&self, _args: &CommandArguments) -> std::io::Result<i32> {
        signal::block(&[SIGINT, SIGTERM])?;

        PatchManager::new()?.save_all_patch_status()?;

        Ok(0)
    }
}
