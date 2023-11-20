use anyhow::{anyhow, ensure, Error, Result};
use log::info;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use crate::{args::SubCommand, flock::ExclusiveFileLockGuard, rpc::RpcProxy};

use super::CommandExecutor;

const PATCH_OP_LOCK_PATH: &str = "/tmp/syscare_patch_op.lock";

pub struct PatchCommandExecutor {
    proxy: RpcProxy,
}

impl PatchCommandExecutor {
    pub fn new(proxy: RpcProxy) -> Self {
        Self { proxy }
    }
}

impl PatchCommandExecutor {
    fn build_error_msg(error_list: impl IntoIterator<Item = Error>) -> Error {
        let mut err_msg = String::new();

        for (idx, e) in error_list.into_iter().enumerate() {
            err_msg.push_str(&format!("{}. {}", idx, e));
            err_msg.push('\n');
            err_msg.push('\n');
        }
        err_msg.pop();
        err_msg.pop();

        anyhow!(err_msg).context("Operation failed")
    }

    fn show_patch_info(patch_list: impl IntoIterator<Item = (String, PatchInfo)>) {
        for (identifier, patch) in patch_list {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", patch);
        }
        info!("-------------------------------------------");
    }

    fn show_patch_target(pkg_list: impl IntoIterator<Item = (String, PackageInfo)>) {
        for (identifier, package) in pkg_list {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", package);
        }
        info!("-------------------------------------------");
    }

    fn show_patch_status(status_list: impl IntoIterator<Item = PatchStateRecord>) {
        for record in status_list {
            info!("{}: {}", record.name, record.status)
        }
    }

    fn show_patch_list(patch_list: impl IntoIterator<Item = PatchListRecord>) {
        info!("{:<40} {:<60} {:<12}", "Uuid", "Name", "Status");
        for record in patch_list {
            info!(
                "{:<40} {:<60} {:<12}",
                record.uuid, record.name, record.status
            )
        }
    }
}

impl CommandExecutor for PatchCommandExecutor {
    fn invoke(&self, command: &SubCommand) -> Result<()> {
        self.check_root_permission()?;

        match command {
            SubCommand::Info { identifiers } => {
                let mut patch_list = vec![];
                let mut error_list = vec![];

                for identifier in identifiers {
                    match self.proxy.get_patch_info(identifier) {
                        Ok(patch) => patch_list.push((identifier.to_owned(), patch)),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_info(patch_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Target { identifiers } => {
                let mut pkg_list = vec![];
                let mut error_list = vec![];

                for identifier in identifiers {
                    match self.proxy.get_patch_target(identifier) {
                        Ok(pkg) => pkg_list.push((identifier.to_owned(), pkg)),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_target(pkg_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Status { identifiers } => {
                let mut status_list = vec![];
                let mut error_list = vec![];

                for identifier in identifiers {
                    match self.proxy.get_patch_status(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::List => {
                Self::show_patch_list(self.proxy.get_patch_list()?);
            }
            SubCommand::Check { identifiers } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut error_list = vec![];
                for identifier in identifiers {
                    if let Err(e) = self.proxy.check_patch(identifier) {
                        error_list.push(e);
                    }
                }

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Apply { identifiers, force } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.apply_patch(identifier, *force) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Remove { identifiers } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.remove_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Active { identifiers } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.active_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Deactive { identifiers } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.deactive_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Accept { identifiers } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.accept_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);

                ensure!(error_list.is_empty(), Self::build_error_msg(error_list));
            }
            SubCommand::Save => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                self.proxy.save_patch_status()?;
            }
            SubCommand::Restore { accepted } => {
                let _flock_guard = ExclusiveFileLockGuard::new(PATCH_OP_LOCK_PATH)?;

                self.proxy.restore_patch_status(*accepted)?;
            }
            _ => {}
        };
        Ok(())
    }
}
