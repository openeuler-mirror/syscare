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

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use indexmap::{indexmap, IndexMap};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn};
use uuid::Uuid;

use syscare_abi::{PatchEntity, PatchInfo, PatchStatus, PatchType, PATCH_INFO_MAGIC};
use syscare_common::{concat_os, ffi::OsStrExt, fs, util::serde};

use crate::{
    config::PatchConfig,
    patch::{
        entity::{KernelPatch, UserPatch},
        PATCH_INFO_FILE_NAME,
    },
};

use super::{
    driver::{PatchDriver, PatchOpFlag},
    entity::Patch,
    PATCH_INSTALL_DIR, PATCH_STATUS_FILE_NAME,
};

type Transition = (PatchStatus, PatchStatus);
type TransitionAction =
    &'static (dyn Fn(&mut PatchManager, &Patch, PatchOpFlag) -> Result<()> + Sync);

const PATCH_CHECK: TransitionAction = &PatchManager::driver_check_patch;
const PATCH_LOAD: TransitionAction = &PatchManager::driver_load_patch;
const PATCH_REMOVE: TransitionAction = &PatchManager::driver_remove_patch;
const PATCH_ACTIVE: TransitionAction = &PatchManager::driver_active_patch;
const PATCH_DEACTIVE: TransitionAction = &PatchManager::driver_deactive_patch;
const PATCH_ACCEPT: TransitionAction = &PatchManager::driver_accept_patch;
const PATCH_DECLINE: TransitionAction = &PatchManager::driver_decline_patch;

lazy_static! {
    static ref STATUS_TRANSITION_MAP: IndexMap<Transition, Vec<TransitionAction>> = indexmap! {
        (PatchStatus::NotApplied, PatchStatus::Deactived) => vec![PATCH_CHECK, PATCH_LOAD],
        (PatchStatus::NotApplied, PatchStatus::Actived) => vec![PATCH_CHECK, PATCH_LOAD, PATCH_ACTIVE],
        (PatchStatus::NotApplied, PatchStatus::Accepted) => vec![PATCH_CHECK, PATCH_LOAD, PATCH_ACTIVE, PATCH_ACCEPT],
        (PatchStatus::Deactived, PatchStatus::NotApplied) => vec![PATCH_REMOVE],
        (PatchStatus::Deactived, PatchStatus::Actived) => vec![PATCH_CHECK, PATCH_ACTIVE],
        (PatchStatus::Deactived, PatchStatus::Accepted) => vec![PATCH_ACTIVE, PATCH_ACCEPT],
        (PatchStatus::Actived, PatchStatus::NotApplied) => vec![PATCH_DEACTIVE, PATCH_REMOVE],
        (PatchStatus::Actived, PatchStatus::Deactived) => vec![PATCH_DEACTIVE],
        (PatchStatus::Actived, PatchStatus::Accepted) => vec![PATCH_ACCEPT],
        (PatchStatus::Accepted, PatchStatus::NotApplied) => vec![PATCH_DECLINE, PATCH_DEACTIVE, PATCH_REMOVE],
        (PatchStatus::Accepted, PatchStatus::Deactived) => vec![PATCH_DECLINE, PATCH_DEACTIVE],
        (PatchStatus::Accepted, PatchStatus::Actived) => vec![PATCH_DECLINE],
    };
}

const PATCH_INIT_RESTORE_ACCEPTED_ONLY: bool = true;

pub struct PatchManager {
    driver: PatchDriver,
    patch_install_dir: PathBuf,
    patch_status_file: PathBuf,
    patch_map: IndexMap<Uuid, Arc<Patch>>,
    status_map: IndexMap<Uuid, PatchStatus>,
}

impl PatchManager {
    pub fn new<P: AsRef<Path>>(patch_config: &PatchConfig, patch_root: P) -> Result<Self> {
        let driver = PatchDriver::new(patch_config)?;
        let patch_install_dir = patch_root.as_ref().join(PATCH_INSTALL_DIR);
        let patch_status_file = patch_root.as_ref().join(PATCH_STATUS_FILE_NAME);
        let patch_map = Self::scan_patches(&patch_install_dir)?;
        let status_map = IndexMap::new();

        let mut instance = Self {
            driver,
            patch_install_dir,
            patch_status_file,
            patch_map,
            status_map,
        };
        instance.restore_patch_status(PATCH_INIT_RESTORE_ACCEPTED_ONLY)?;

        Ok(instance)
    }

    fn finallize(&mut self) {
        if let Err(e) = self.save_patch_status() {
            error!("{:?}", e)
        }
    }
}

impl PatchManager {
    pub fn match_patch(&self, identifier: &str) -> Result<Vec<Arc<Patch>>> {
        debug!("Matching patch by '{}'...", identifier);
        if let Ok(uuid) = Uuid::from_str(identifier) {
            if let Ok(patch) = self.find_patch_by_uuid(&uuid) {
                debug!("Matched '{}'", patch);
                debug!("Matched 1 patch");
                return Ok(vec![patch]);
            }
        }

        let patch_list = self.find_patch_by_name(identifier)?;
        for patch in &patch_list {
            debug!("Matched '{}'", patch)
        }
        debug!("Matched {} patch(es)", patch_list.len());

        Ok(patch_list)
    }

    pub fn get_patch_list(&self) -> Vec<Arc<Patch>> {
        self.patch_map.values().cloned().collect()
    }

    pub fn get_patch_status(&mut self, patch: &Patch) -> Result<PatchStatus> {
        let mut status = self
            .status_map
            .get(patch.uuid())
            .copied()
            .unwrap_or_default();

        if status == PatchStatus::Unknown {
            status = self.driver_get_patch_status(patch)?;
            self.set_patch_status(patch, status)?;
        }

        Ok(status)
    }

    pub fn check_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        info!("Check patch '{}'", patch);
        self.driver.check_patch(patch, flag)?;
        self.driver.check_confliction(patch, flag)?;

        Ok(())
    }

    pub fn apply_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus> {
        info!("Apply patch '{}'", patch);
        let current_status = self.get_patch_status(patch)?;

        // Not-Applied -> Actived
        if current_status == PatchStatus::Actived {
            return Ok(current_status);
        }

        self.do_status_transition(patch, PatchStatus::Actived, flag)
    }

    pub fn remove_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus> {
        info!("Remove patch '{}'", patch);
        let current_status = self.get_patch_status(patch)?;

        // Deactived -> Not-Applied
        if current_status == PatchStatus::NotApplied {
            return Ok(PatchStatus::NotApplied);
        }

        self.do_status_transition(patch, PatchStatus::NotApplied, flag)
    }

    pub fn active_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus> {
        info!("Active patch '{}'", patch);
        let current_status = self.get_patch_status(patch)?;

        // Deactived -> Actived
        if current_status == PatchStatus::Actived {
            return Ok(PatchStatus::Actived);
        }
        if current_status < PatchStatus::Deactived {
            bail!("Patch '{}' is not applied", patch);
        }

        self.do_status_transition(patch, PatchStatus::Actived, flag)
    }

    pub fn deactive_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus> {
        info!("Deactive patch '{}'", patch);
        let current_status = self.get_patch_status(patch)?;

        // Actived -> Deactived
        if current_status == PatchStatus::Deactived {
            return Ok(PatchStatus::Deactived);
        }
        if current_status < PatchStatus::Actived {
            bail!("Patch '{}' is not actived", patch);
        }

        self.do_status_transition(patch, PatchStatus::Deactived, flag)
    }

    pub fn accept_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<PatchStatus> {
        info!("Accept patch '{}'", patch);
        let current_status = self.get_patch_status(patch)?;

        // Actived -> Accepted
        if current_status == PatchStatus::Accepted {
            return Ok(PatchStatus::Accepted);
        }
        if current_status != PatchStatus::Actived {
            bail!("Patch '{}' is not actived", patch);
        }

        self.do_status_transition(patch, PatchStatus::Accepted, flag)
    }

    pub fn save_patch_status(&mut self) -> Result<()> {
        info!("Saving all patch status...");

        debug!("Updating patch status...");
        for patch in self.get_patch_list() {
            self.get_patch_status(&patch)?;
        }

        debug!("Writing patch status...");
        for (uuid, status) in &self.status_map {
            debug!("Patch '{}' status: {}", uuid, status);
        }
        serde::serialize(&self.status_map, &self.patch_status_file)
            .context("Failed to write patch status file")?;

        fs::sync();

        info!("All patch status were saved");
        Ok(())
    }

    pub fn restore_patch_status(&mut self, accepted_only: bool) -> Result<()> {
        info!("Restoring all patch status...");
        if !self.patch_status_file.exists() {
            return Ok(());
        }

        debug!("Reading patch status...");
        let status_map: IndexMap<Uuid, PatchStatus> =
            serde::deserialize(&self.patch_status_file)
                .context("Failed to read patch status file")?;
        for (uuid, status) in &status_map {
            debug!("Patch '{}' status: {}", uuid, status);
        }
        for (uuid, status) in status_map {
            if accepted_only && (status != PatchStatus::Accepted) {
                continue;
            }
            match self.find_patch_by_uuid(&uuid) {
                Ok(patch) => {
                    info!("Restore patch '{}' status to '{}'", patch, status);
                    if let Err(e) = self.do_status_transition(&patch, status, PatchOpFlag::Force) {
                        error!("{:?}", e);
                    }
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
        }

        info!("All patch status were restored");
        Ok(())
    }

    pub fn rescan_patches(&mut self) -> Result<()> {
        self.patch_map = Self::scan_patches(&self.patch_install_dir)?;

        let status_keys = self.status_map.keys().copied().collect::<Vec<_>>();
        for patch_uuid in status_keys {
            if !self.patch_map.contains_key(&patch_uuid) {
                trace!("Patch '{}' was removed", patch_uuid);
                self.status_map.remove(&patch_uuid);
            }
        }

        Ok(())
    }

    pub(super) fn do_status_transition(
        &mut self,
        patch: &Patch,
        status: PatchStatus,
        flag: PatchOpFlag,
    ) -> Result<PatchStatus> {
        let current_status = self.get_patch_status(patch)?;
        let target_status = status;
        if current_status == target_status {
            debug!(
                "Patch '{}': Current status is already '{}'",
                patch, target_status,
            );
            return Ok(target_status);
        }

        match STATUS_TRANSITION_MAP.get(&(current_status, target_status)) {
            Some(action_list) => {
                debug!(
                    "Patch '{}': Switching status from '{}' to '{}'",
                    patch, current_status, status
                );
                for action in action_list {
                    action(self, patch, flag)?;
                }
            }
            None => {
                warn!(
                    "Patch '{}': Ignored invalid status transition from '{}' to '{}'",
                    patch, current_status, status
                );
            }
        }

        let new_status = self.get_patch_status(patch)?;
        if new_status != status {
            bail!("Patch '{}' does not reach '{}' status", patch, status);
        }

        Ok(new_status)
    }
}

impl PatchManager {
    fn parse_user_patch(
        root_dir: &Path,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
    ) -> Result<UserPatch> {
        let patch_name = concat_os!(
            patch_info.target.short_name(),
            "/",
            patch_info.name(),
            "/",
            patch_entity.patch_target.file_name().unwrap_or_default()
        );
        let patch_file = root_dir.join(&patch_entity.patch_name);

        let patch = UserPatch::parse(&patch_name, patch_info, patch_entity, patch_file)
            .with_context(|| {
                format!(
                    "Failed to parse patch '{}' ({})",
                    patch_entity.uuid,
                    patch_name.to_string_lossy(),
                )
            })?;

        debug!("Found patch '{}' ({})", patch.uuid, patch);
        Ok(patch)
    }

    fn parse_kernel_patch(
        root_dir: &Path,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
    ) -> Result<KernelPatch> {
        const KPATCH_EXTENSION: &str = "ko";

        let patch_name = concat_os!(
            patch_info.target.short_name(),
            "/",
            patch_info.name(),
            "/",
            &patch_entity.patch_target,
        );
        let mut patch_file = root_dir.join(&patch_entity.patch_name);
        patch_file.set_extension(KPATCH_EXTENSION);

        let patch = KernelPatch::parse(&patch_name, patch_info, patch_entity, patch_file)
            .with_context(|| {
                format!(
                    "Failed to parse patch '{}' ({})",
                    patch_entity.uuid,
                    patch_name.to_string_lossy(),
                )
            })?;

        debug!("Found patch '{}' ({})", patch.uuid, patch);
        Ok(patch)
    }

    fn parse_patches(root_dir: &Path) -> Result<Vec<Patch>> {
        let root_name = root_dir.file_name().expect("Invalid patch root directory");
        let patch_metadata = root_dir.join(PATCH_INFO_FILE_NAME);
        let patch_info = Arc::new(
            serde::deserialize_with_magic::<PatchInfo, _, _>(patch_metadata, PATCH_INFO_MAGIC)
                .with_context(|| {
                    format!(
                        "Failed to parse patch '{}' metadata",
                        root_name.to_string_lossy(),
                    )
                })?,
        );

        patch_info
            .entities
            .iter()
            .map(|patch_entity| {
                let patch_info = patch_info.clone();
                match patch_info.kind {
                    PatchType::UserPatch => Ok(Patch::UserPatch(Self::parse_user_patch(
                        root_dir,
                        patch_info,
                        patch_entity,
                    )?)),
                    PatchType::KernelPatch => Ok(Patch::KernelPatch(Self::parse_kernel_patch(
                        root_dir,
                        patch_info,
                        patch_entity,
                    )?)),
                }
            })
            .collect::<Result<Vec<_>>>()
    }

    fn scan_patches<P: AsRef<Path>>(directory: P) -> Result<IndexMap<Uuid, Arc<Patch>>> {
        const TRAVERSE_OPTION: fs::TraverseOptions = fs::TraverseOptions { recursive: false };

        let mut patch_map = IndexMap::new();

        info!(
            "Scanning patches from '{}'...",
            directory.as_ref().display()
        );
        for root_dir in fs::list_dirs(directory, TRAVERSE_OPTION)? {
            match Self::parse_patches(&root_dir) {
                Ok(patches) => {
                    for patch in patches {
                        patch_map.insert(*patch.uuid(), Arc::new(patch));
                    }
                }
                Err(e) => error!("{:?}", e),
            }
        }

        patch_map.sort_by(|_, lhs, _, rhs| lhs.cmp(rhs));
        info!("Found {} patch(es)", patch_map.len());

        Ok(patch_map)
    }

    fn find_patch_by_uuid(&self, uuid: &Uuid) -> Result<Arc<Patch>> {
        self.patch_map
            .get(uuid)
            .cloned()
            .with_context(|| format!("Cannot find patch by '{}'", uuid))
    }

    fn find_patch_by_name(&self, identifier: &str) -> Result<Vec<Arc<Patch>>> {
        let match_result = self
            .patch_map
            .values()
            .filter(|patch| {
                let entity_name = patch.name();
                if identifier == entity_name {
                    return true;
                }

                let fields = entity_name.split('/').collect::<Vec<_>>();
                let patch_name = concat_os!(fields[0], "/", fields[1]);
                if identifier == patch_name {
                    return true;
                }

                let pkg_name = patch.pkg_name();
                if identifier == pkg_name {
                    return true;
                }

                false
            })
            .cloned()
            .collect::<Vec<_>>();

        if match_result.is_empty() {
            bail!("Cannot match any patch named '{}'", identifier);
        }
        Ok(match_result)
    }

    fn set_patch_status(&mut self, patch: &Patch, value: PatchStatus) -> Result<()> {
        if value == PatchStatus::Unknown {
            bail!("Cannot set patch '{}' status to '{}'", patch, value);
        }

        let uuid = *patch.uuid();
        let (curr_index, _) = self.status_map.insert_full(uuid, value);

        let last_index = self.status_map.len().saturating_sub(1);
        if curr_index != last_index {
            self.status_map.move_index(curr_index, last_index);
        }

        Ok(())
    }
}

impl PatchManager {
    fn driver_check_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.check_patch(patch, flag)
    }

    fn driver_get_patch_status(&self, patch: &Patch) -> Result<PatchStatus> {
        self.driver.get_patch_status(patch)
    }

    fn driver_load_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.driver.load_patch(patch)?;
        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn driver_remove_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.driver.remove_patch(patch)?;
        self.set_patch_status(patch, PatchStatus::NotApplied)
    }

    fn driver_active_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.active_patch(patch, flag)?;
        self.set_patch_status(patch, PatchStatus::Actived)
    }

    fn driver_deactive_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.deactive_patch(patch, flag)?;
        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn driver_accept_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.set_patch_status(patch, PatchStatus::Accepted)
    }

    fn driver_decline_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.set_patch_status(patch, PatchStatus::Actived)
    }
}

impl Drop for PatchManager {
    fn drop(&mut self) {
        self.finallize()
    }
}
