// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{fmt::Write, path::PathBuf};

use anyhow::{anyhow, Context, Error, Result};
use log::info;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};
use syscare_common::fs::{FileLock, FileLockType};

use crate::{args::SubCommand, rpc::RpcProxy};

use super::CommandExecutor;

pub struct PatchCommandExecutor {
    proxy: RpcProxy,
    lock_file: PathBuf,
}

impl PatchCommandExecutor {
    pub fn new(proxy: RpcProxy, lock_file: PathBuf) -> Self {
        Self { proxy, lock_file }
    }
}

impl PatchCommandExecutor {
    fn check_error(mut error_list: Vec<Error>) -> Result<()> {
        match error_list.len() {
            0 => Ok(()),
            1 => Err(error_list.pop().context("Invalid error")?),
            _ => {
                let mut err_msg = String::new();
                for (idx, e) in error_list.into_iter().enumerate() {
                    writeln!(err_msg, "{}. {}", idx, e)?;
                }
                err_msg.pop();

                Err(anyhow!(err_msg))
            }
        }
    }

    fn show_patch_info(patch_list: impl IntoIterator<Item = (String, PatchInfo)>) {
        let mut patch_iter = patch_list.into_iter().peekable();
        while let Some((identifier, patch)) = patch_iter.next() {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", patch);
            if patch_iter.peek().is_some() {
                continue;
            }
            info!("-------------------------------------------");
        }
    }

    fn show_patch_target(pkg_list: impl IntoIterator<Item = (String, PackageInfo)>) {
        let mut pkg_iter = pkg_list.into_iter().peekable();
        while let Some((identifier, package)) = pkg_iter.next() {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", package);
            if pkg_iter.peek().is_some() {
                continue;
            }
            info!("-------------------------------------------");
        }
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
    fn invoke(&self, command: &SubCommand) -> Result<Option<i32>> {
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
                Self::check_error(error_list)?;

                return Ok(Some(0));
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
                Self::check_error(error_list)?;

                return Ok(Some(0));
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
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::List => {
                Self::show_patch_list(self.proxy.get_patch_list()?);
                return Ok(Some(0));
            }
            SubCommand::Check { identifiers } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut error_list = vec![];
                for identifier in identifiers {
                    if let Err(e) = self.proxy.check_patch(identifier) {
                        error_list.push(e);
                    }
                }
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Apply { identifiers, force } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.apply_patch(identifier, *force) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Remove { identifiers } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.remove_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Active { identifiers, force } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.active_patch(identifier, *force) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Deactive { identifiers } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.deactive_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Accept { identifiers } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                let mut status_list = vec![];
                let mut error_list = vec![];
                for identifier in identifiers {
                    match self.proxy.accept_patch(identifier) {
                        Ok(new_status) => status_list.extend(new_status),
                        Err(e) => error_list.push(e),
                    }
                }
                Self::show_patch_status(status_list);
                Self::check_error(error_list)?;

                return Ok(Some(0));
            }
            SubCommand::Save => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                self.proxy.save_patch_status()?;
                return Ok(Some(0));
            }
            SubCommand::Restore { accepted } => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                self.proxy.restore_patch_status(*accepted)?;
                return Ok(Some(0));
            }
            SubCommand::Rescan => {
                let _file_lock = FileLock::new(&self.lock_file, FileLockType::Exclusive)?;

                self.proxy.rescan_patches()?;
                return Ok(Some(0));
            }
            _ => {}
        }

        Ok(None)
    }
}
