use anyhow::Result;

use crate::{args::SubCommand, rpc::RebootProxy};

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
    fn invoke(&self, command: &SubCommand) -> Result<()> {
        self.check_root_permission()?;
        if let SubCommand::Reboot { target, force } = command {
            self.proxy.fast_reboot(target.to_owned(), *force)?;
        }
        Ok(())
    }
}
