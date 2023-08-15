use anyhow::Result;
use log::info;

use syscared::abi::patch::{PatchInfo, PatchListRecord, PatchStateRecord, PatchTargetRecord};

use crate::{args::CliCommand, proxy::PatchProxy};

use super::CommandExecutor;

pub struct PatchCommandExecutor {
    proxy: PatchProxy,
}

impl PatchCommandExecutor {
    pub fn new(proxy: PatchProxy) -> Self {
        Self { proxy }
    }
}

impl PatchCommandExecutor {
    fn show_patch_info(patch_info: PatchInfo) {
        info!("{:?}", patch_info)
    }

    fn show_patch_state(list: impl IntoIterator<Item = PatchStateRecord>) {
        for record in list {
            info!("{}: {}", record.name, record.status)
        }
    }

    fn show_patch_target(list: impl IntoIterator<Item = PatchTargetRecord>) {
        for record in list {
            info!("{}: {}", record.name, record.target.full_name())
        }
    }

    fn show_patch_list(list: impl IntoIterator<Item = PatchListRecord>) {
        info!("{:<40} {:<60} {:<12}", "Uuid", "Name", "Status");
        for record in list {
            info!(
                "{:<40} {:<60} {:<12}",
                record.uuid, record.name, record.status
            )
        }
    }
}

impl CommandExecutor for PatchCommandExecutor {
    fn invoke(&self, command: &CliCommand) -> Result<()> {
        match command {
            CliCommand::Info { identifier } => {
                let patch_info = self.proxy.get_patch_info(identifier)?;
                Self::show_patch_info(patch_info);
            }
            CliCommand::Target { identifier } => {
                let target_list = self.proxy.get_patch_target(identifier)?;
                Self::show_patch_target(target_list);
            }
            CliCommand::Status { identifier } => {
                let result_list = self.proxy.get_patch_status(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::List => {
                let patch_list = self.proxy.get_patch_list()?;
                Self::show_patch_list(patch_list);
            }
            CliCommand::Apply { identifier } => {
                let result_list = self.proxy.apply_patch(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::Remove { identifier } => {
                let result_list = self.proxy.remove_patch(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::Active { identifier } => {
                let result_list = self.proxy.active_patch(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::Deactive { identifier } => {
                let result_list = self.proxy.deactive_patch(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::Accept { identifier } => {
                let result_list = self.proxy.accept_patch(identifier)?;
                Self::show_patch_state(result_list);
            }
            CliCommand::Save => {
                self.proxy.save_patch_status()?;
            }
            CliCommand::Restore { accepted } => {
                let accepted_only = *accepted;
                self.proxy.restore_patch_status(accepted_only)?;
            }
            _ => {}
        };
        Ok(())
    }
}
