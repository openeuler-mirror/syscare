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

use std::fmt::Write;

use anyhow::{anyhow, Error, Result};
use function_name::named;

use log::info;
use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use super::client::{RpcArguments, RpcClient};

pub struct PatchProxy<'a> {
    client: &'a RpcClient,
}

impl<'a> PatchProxy<'a> {
    pub fn new(client: &'a RpcClient) -> Self {
        Self { client }
    }
}

/* RPC methods */
impl PatchProxy<'_> {
    #[named]
    fn check_patch(&self, identifier: &str) -> Result<()> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn apply_patch(&self, identifier: &str, force: bool) -> Result<Vec<PatchStateRecord>> {
        self.client.call_with_args(
            function_name!(),
            RpcArguments::new().arg(identifier).arg(force),
        )
    }

    #[named]
    fn remove_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn active_patch(&self, identifier: &str, force: bool) -> Result<Vec<PatchStateRecord>> {
        self.client.call_with_args(
            function_name!(),
            RpcArguments::new().arg(identifier).arg(force),
        )
    }

    #[named]
    fn deactive_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn accept_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn get_patch_list(&self) -> Result<Vec<PatchListRecord>> {
        self.client.call(function_name!())
    }

    #[named]
    fn get_patch_status(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn get_patch_info(&self, identifier: &str) -> Result<PatchInfo> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn get_patch_target(&self, identifier: &str) -> Result<PackageInfo> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    fn save_patch_status(&self) -> Result<()> {
        self.client.call(function_name!())
    }

    #[named]
    fn restore_patch_status(&self, accepted_only: bool) -> Result<()> {
        self.client
            .call_with_args(function_name!(), RpcArguments::new().arg(accepted_only))
    }

    #[named]
    fn rescan_patches(&self) -> Result<()> {
        self.client.call(function_name!())
    }
}

/* Internal methods */
impl PatchProxy<'_> {
    fn check_error(mut errors: Vec<Error>) -> Result<()> {
        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.remove(0)),
            _ => {
                let mut err_msg = String::new();
                for (idx, e) in errors.into_iter().enumerate() {
                    writeln!(err_msg, "{}. {}", idx, e)?;
                }
                err_msg.pop();

                Err(anyhow!(err_msg))
            }
        }
    }
}

/* External methods */
impl PatchProxy<'_> {
    pub fn show_patch_info(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];

        for identifier in identifiers {
            match self.get_patch_info(identifier) {
                Ok(patch_info) => results.push((identifier.as_str(), patch_info)),
                Err(e) => errors.push(e),
            }
        }

        let mut result_iter = results.into_iter().peekable();
        while let Some((identifier, patch_info)) = result_iter.next() {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", patch_info);
            if result_iter.peek().is_some() {
                continue;
            }
            info!("-------------------------------------------");
        }
        Self::check_error(errors)
    }

    pub fn show_patch_target(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];

        for identifier in identifiers {
            match self.get_patch_target(identifier) {
                Ok(pkg_info) => results.push((identifier.as_str(), pkg_info)),
                Err(e) => errors.push(e),
            }
        }

        let mut result_iter = results.into_iter().peekable();
        while let Some((identifier, pkg_info)) = result_iter.next() {
            info!("-------------------------------------------");
            info!("Patch: {}", identifier);
            info!("-------------------------------------------");
            info!("{}", pkg_info);
            if result_iter.peek().is_some() {
                continue;
            }
            info!("-------------------------------------------");
        }
        Self::check_error(errors)
    }

    pub fn show_patch_status(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];

        for identifier in identifiers {
            match self.get_patch_status(identifier) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for record in results {
            info!("{}: {}", record.name, record.status)
        }
        Self::check_error(errors)
    }

    pub fn show_patch_list(&self) -> Result<()> {
        let _ = self.client.lock()?;

        let patch_list = self.get_patch_list()?;

        info!("{:<40} {:<60} {:<12}", "Uuid", "Name", "Status");
        for patch in patch_list {
            info!("{:<40} {:<60} {:<12}", patch.uuid, patch.name, patch.status)
        }

        Ok(())
    }

    pub fn check_patches(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut errors = vec![];
        for identifier in identifiers {
            if let Err(e) = self.check_patch(identifier) {
                errors.push(e);
            }
        }
        Self::check_error(errors)
    }

    pub fn apply_patches(&self, identifiers: &[String], force: bool) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];
        for identifier in identifiers {
            match self.apply_patch(identifier, force) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for result in results {
            info!("{}: {}", result.name, result.status);
        }
        Self::check_error(errors)
    }

    pub fn remove_patches(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];
        for identifier in identifiers {
            match self.remove_patch(identifier) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for result in results {
            info!("{}: {}", result.name, result.status);
        }
        Self::check_error(errors)
    }

    pub fn active_patches(&self, identifiers: &[String], force: bool) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];
        for identifier in identifiers {
            match self.active_patch(identifier, force) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for result in results {
            info!("{}: {}", result.name, result.status);
        }
        Self::check_error(errors)
    }

    pub fn deactive_patches(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];
        for identifier in identifiers {
            match self.deactive_patch(identifier) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for result in results {
            info!("{}: {}", result.name, result.status);
        }
        Self::check_error(errors)
    }

    pub fn accept_patches(&self, identifiers: &[String]) -> Result<()> {
        let _ = self.client.lock()?;

        let mut results = vec![];
        let mut errors = vec![];
        for identifier in identifiers {
            match self.accept_patch(identifier) {
                Ok(status_list) => results.extend(status_list),
                Err(e) => errors.push(e),
            }
        }

        for result in results {
            info!("{}: {}", result.name, result.status);
        }
        Self::check_error(errors)
    }

    pub fn save_patches(&self) -> Result<()> {
        let _ = self.client.lock()?;
        self.save_patch_status()
    }

    pub fn restore_patches(&self, accepted_only: bool) -> Result<()> {
        let _ = self.client.lock()?;
        self.restore_patch_status(accepted_only)
    }

    pub fn rescan_all_patches(&self) -> Result<()> {
        let _ = self.client.lock()?;
        self.rescan_patches()
    }
}
