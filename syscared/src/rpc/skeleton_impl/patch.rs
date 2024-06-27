// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::sync::Arc;

use anyhow::{Context, Result};

use parking_lot::RwLock;
use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use crate::patch::{
    driver::PatchOpFlag, entity::Patch, manager::PatchManager, transaction::PatchTransaction,
};

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::PatchSkeleton,
};

pub struct PatchSkeletonImpl {
    patch_manager: Arc<RwLock<PatchManager>>,
}

impl PatchSkeletonImpl {
    pub fn new(patch_manager: Arc<RwLock<PatchManager>>) -> Self {
        Self { patch_manager }
    }
}

impl PatchSkeletonImpl {
    fn normalize_identifier(identifier: &mut String) {
        while identifier.ends_with('/') {
            identifier.pop();
        }
    }

    fn parse_state_record(&self, patch: Arc<Patch>) -> Result<PatchStateRecord> {
        let patch_name = patch.to_string();
        let patch_status = self
            .patch_manager
            .write()
            .get_patch_status(&patch)
            .unwrap_or_default();

        Ok(PatchStateRecord {
            name: patch_name,
            status: patch_status,
        })
    }

    fn parse_list_record(&self, patch: Arc<Patch>) -> Result<PatchListRecord> {
        let patch_uuid = patch.uuid().to_string();
        let patch_name = patch.to_string();
        let patch_status = self
            .patch_manager
            .write()
            .get_patch_status(&patch)
            .unwrap_or_default();

        Ok(PatchListRecord {
            uuid: patch_uuid,
            name: patch_name,
            status: patch_status,
        })
    }
}

impl PatchSkeleton for PatchSkeletonImpl {
    fn check_patch(&self, mut identifier: String) -> RpcResult<()> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<()> {
            let mut patch_manager = self.patch_manager.write();
            for patch in patch_manager.match_patch(&identifier)? {
                patch_manager.check_patch(&patch, PatchOpFlag::Normal)?;
            }

            Ok(())
        })
    }

    fn apply_patch(&self, mut identifier: String, force: bool) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Apply patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::apply_patch,
                if force {
                    PatchOpFlag::Force
                } else {
                    PatchOpFlag::Normal
                },
                identifier,
            )
            .invoke()
        })
    }

    fn remove_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Remove patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::remove_patch,
                PatchOpFlag::Normal,
                identifier,
            )
            .invoke()
        })
    }

    fn active_patch(
        &self,
        mut identifier: String,
        force: bool,
    ) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Active patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::active_patch,
                if force {
                    PatchOpFlag::Force
                } else {
                    PatchOpFlag::Normal
                },
                identifier,
            )
            .invoke()
        })
    }

    fn deactive_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Deactive patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::deactive_patch,
                PatchOpFlag::Normal,
                identifier,
            )
            .invoke()
        })
    }

    fn accept_patch(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            PatchTransaction::new(
                format!("Accept patch '{}'", identifier),
                self.patch_manager.clone(),
                PatchManager::accept_patch,
                PatchOpFlag::Normal,
                identifier,
            )
            .invoke()
        })
    }

    fn get_patch_list(&self) -> RpcResult<Vec<PatchListRecord>> {
        RpcFunction::call(move || -> Result<Vec<PatchListRecord>> {
            let patch_list: Vec<Arc<Patch>> = self.patch_manager.read().get_patch_list();

            let mut result = Vec::new();
            for patch in patch_list {
                result.push(self.parse_list_record(patch)?);
            }

            Ok(result)
        })
    }

    fn get_patch_status(&self, mut identifier: String) -> RpcResult<Vec<PatchStateRecord>> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<Vec<PatchStateRecord>> {
            let patch_list = self.patch_manager.read().match_patch(&identifier)?;

            let mut result = Vec::new();
            for patch in patch_list {
                result.push(self.parse_state_record(patch)?);
            }
            Ok(result)
        })
    }

    fn get_patch_info(&self, mut identifier: String) -> RpcResult<PatchInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PatchInfo> {
            let patch_list = self.patch_manager.read().match_patch(&identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info().clone())
        })
    }

    fn get_patch_target(&self, mut identifier: String) -> RpcResult<PackageInfo> {
        Self::normalize_identifier(&mut identifier);
        RpcFunction::call(move || -> Result<PackageInfo> {
            let patch_list = self.patch_manager.read().match_patch(&identifier)?;
            let patch = patch_list.first().context("No patch matched")?;

            Ok(patch.info().target.clone())
        })
    }

    fn save_patch_status(&self) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            self.patch_manager
                .write()
                .save_patch_status()
                .context("Failed to save patch status")
        })
    }

    fn restore_patch_status(&self, accepted_only: bool) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            self.patch_manager
                .write()
                .restore_patch_status(accepted_only)
                .context("Failed to restore patch status")
        })
    }
}
