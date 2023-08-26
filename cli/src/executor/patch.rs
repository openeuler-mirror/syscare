use anyhow::Result;
use log::info;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};
use syscare_common::util::fs;

use crate::{args::CliCommand, rpc::PatchProxy};

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
        const PATCH_FLAG_NONE: &str = "(none)";

        let patch_elfs = match patch_info.entities.is_empty() {
            true => PATCH_FLAG_NONE.to_owned(),
            false => patch_info
                .entities
                .iter()
                .map(|entity| {
                    format!(
                        "{}, ",
                        fs::file_name(&entity.patch_target).to_string_lossy()
                    )
                })
                .collect::<String>()
                .trim_end_matches(", ")
                .to_string(),
        };

        info!("uuid:        {}", patch_info.uuid);
        info!("name:        {}", patch_info.name);
        info!("version:     {}", patch_info.version);
        info!("release:     {}", patch_info.release);
        info!("arch:        {}", patch_info.arch);
        info!("type:        {}", patch_info.kind);
        info!("target:      {}", patch_info.target.short_name());
        info!("target_elf:  {}", patch_elfs);
        info!("license:     {}", patch_info.target.license);
        info!("description: {}", patch_info.description);
        info!("patch:");
        for patch_file in patch_info.patches {
            info!("{}", patch_file.name.to_string_lossy())
        }
    }

    fn show_patch_target(package: PackageInfo) {
        info!("name:    {}", package.name);
        info!("type:    {}", package.kind);
        info!("arch:    {}", package.arch);
        info!("epoch:   {}", package.epoch);
        info!("version: {}", package.version);
        info!("release: {}", package.release);
        info!("license: {}", package.license);
    }

    fn show_patch_state(list: impl IntoIterator<Item = PatchStateRecord>) {
        for record in list {
            info!("{}: {}", record.name, record.status)
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
        self.check_root_permission()?;
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
