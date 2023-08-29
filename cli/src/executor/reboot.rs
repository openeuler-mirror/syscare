use anyhow::Result;

use crate::{args::CliCommand, rpc::RebootProxy};

use super::CommandExecutor;

pub struct RebootCommandExecutor {
    proxy: RebootProxy,
}

impl RebootCommandExecutor {
    pub fn new(proxy: RebootProxy) -> Self {
        Self { proxy }
    }
}

impl CommandExecutor for RebootCommandExecutor {
    fn invoke(&self, command: &CliCommand) -> Result<()> {
        self.check_root_permission()?;
        if let CliCommand::Reboot { target, force } = command {
            self.proxy.fast_reboot(target.to_owned(), *force)?;
        }
        Ok(())
    }
}
