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
    cmp::Ordering,
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use indexmap::{indexmap, IndexMap};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn};
use uuid::Uuid;

use syscare_abi::PatchStatus;
use syscare_common::{concat_os, ffi::OsStrExt, fs, util::serde};

use crate::patch::resolver::PatchResolver;

use super::{
    driver::{PatchDriver, PatchOpFlag},
    entity::Patch,
    PATCH_INSTALL_DIR, PATCH_STATUS_FILE_NAME,
};

type Transition = (PatchStatus, PatchStatus);
type TransitionAction =
    &'static (dyn Fn(&mut PatchManager, &Patch, PatchOpFlag) -> Result<()> + Sync);

const PATCH_CHECK: TransitionAction = &PatchManager::driver_check_patch;
const PATCH_APPLY: TransitionAction = &PatchManager::driver_apply_patch;
const PATCH_REMOVE: TransitionAction = &PatchManager::driver_remove_patch;
const PATCH_ACTIVE: TransitionAction = &PatchManager::driver_active_patch;
const PATCH_DEACTIVE: TransitionAction = &PatchManager::driver_deactive_patch;
const PATCH_ACCEPT: TransitionAction = &PatchManager::driver_accept_patch;

lazy_static! {
    static ref STATUS_TRANSITION_MAP: IndexMap<Transition, Vec<TransitionAction>> = indexmap! {
        (PatchStatus::NotApplied, PatchStatus::Deactived) => vec![PATCH_CHECK, PATCH_APPLY],
        (PatchStatus::NotApplied, PatchStatus::Actived) => vec![PATCH_CHECK, PATCH_APPLY, PATCH_ACTIVE],
        (PatchStatus::NotApplied, PatchStatus::Accepted) => vec![PATCH_CHECK, PATCH_APPLY, PATCH_ACTIVE, PATCH_ACCEPT],
        (PatchStatus::Deactived, PatchStatus::NotApplied) => vec![PATCH_REMOVE],
        (PatchStatus::Deactived, PatchStatus::Actived) => vec![PATCH_ACTIVE],
        (PatchStatus::Deactived, PatchStatus::Accepted) => vec![PATCH_ACTIVE, PATCH_ACCEPT],
        (PatchStatus::Actived, PatchStatus::NotApplied) => vec![PATCH_DEACTIVE, PATCH_REMOVE],
        (PatchStatus::Actived, PatchStatus::Deactived) => vec![PATCH_DEACTIVE],
        (PatchStatus::Actived, PatchStatus::Accepted) => vec![PATCH_ACCEPT],
        (PatchStatus::Accepted, PatchStatus::NotApplied) => vec![PATCH_ACCEPT, PATCH_DEACTIVE, PATCH_REMOVE],
        (PatchStatus::Accepted, PatchStatus::Deactived) => vec![PATCH_ACCEPT, PATCH_DEACTIVE],
        (PatchStatus::Accepted, PatchStatus::Actived) => vec![PATCH_ACCEPT],
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
    pub fn new<P: AsRef<Path>>(patch_root: P) -> Result<Self> {
        let driver = PatchDriver::new()?;
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
            status = self.driver_get_patch_status(patch, PatchOpFlag::Normal)?;
            self.set_patch_status(patch, status)?;
        }

        Ok(status)
    }

    pub fn check_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver_check_patch(patch, flag)
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

        debug!("Updating all patch status...");
        for patch in self.get_patch_list() {
            debug!("Update patch '{}' status", patch);
            self.get_patch_status(&patch)?;
        }

        let mut status_map = HashMap::new();
        for (uuid, status) in &self.status_map {
            status_map.insert(uuid, status);
        }

        debug!("Writing patch status file");
        serde::serialize(&status_map, &self.patch_status_file)
            .context("Failed to write patch status file")?;

        fs::sync();

        info!("All patch status were saved");
        Ok(())
    }

    pub fn restore_patch_status(&mut self, accepted_only: bool) -> Result<()> {
        info!("Restoring all patch status...");

        debug!("Reading patch status...");
        let status_file = &self.patch_status_file;
        let status_map: HashMap<Uuid, PatchStatus> = match status_file.exists() {
            true => serde::deserialize(status_file).context("Failed to read patch status")?,
            false => {
                warn!("Cannot find patch status file");
                return Ok(());
            }
        };

        /*
         * To ensure that we won't load multiple patches for same target at the same time,
         * we take a sort operation of the status to make sure do REMOVE operation at first
         */
        let mut restore_list = status_map
            .into_iter()
            .filter_map(|(uuid, status)| match self.find_patch_by_uuid(&uuid) {
                Ok(patch) => {
                    if accepted_only && (status != PatchStatus::Accepted) {
                        debug!(
                            "Skipped patch '{}', status is not '{}'",
                            patch,
                            PatchStatus::Accepted
                        );
                        return None;
                    }
                    Some((patch, status))
                }
                Err(e) => {
                    error!("{:?}", e);
                    None
                }
            })
            .collect::<Vec<_>>();

        restore_list.sort_by(|(lhs_patch, lhs_status), (rhs_patch, rhs_status)| {
            match lhs_status.cmp(rhs_status) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => lhs_patch.cmp(rhs_patch),
                Ordering::Greater => Ordering::Greater,
            }
        });

        for (patch, target_status) in restore_list {
            debug!("Restore patch '{}' status to '{}'", patch, target_status);
            if let Err(e) = self.do_status_transition(&patch, target_status, PatchOpFlag::Force) {
                error!("{}", e);
            }
        }
        info!("All patch status were restored");

        Ok(())
    }

    pub fn rescan_patches(&mut self) -> Result<()> {
        self.patch_map = Self::scan_patches(&self.patch_install_dir)?;

        let status_keys = self.status_map.keys().cloned().collect::<Vec<_>>();
        for patch_uuid in status_keys {
            if !self.patch_map.contains_key(&patch_uuid) {
                trace!("Patch '{}' was removed, remove its status", patch_uuid);
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
    fn scan_patches<P: AsRef<Path>>(directory: P) -> Result<IndexMap<Uuid, Arc<Patch>>> {
        const TRAVERSE_OPTION: fs::TraverseOptions = fs::TraverseOptions { recursive: false };

        let mut patch_map = IndexMap::new();

        info!("Scanning patches from {}...", directory.as_ref().display());
        for patch_root in fs::list_dirs(directory, TRAVERSE_OPTION)? {
            let resolve_result = PatchResolver::resolve_patch(&patch_root)
                .with_context(|| format!("Failed to resolve patch from {}", patch_root.display()));
            match resolve_result {
                Ok(patches) => {
                    for patch in patches {
                        debug!("Detected patch '{}'", patch);
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

        let status_map = &mut self.status_map;
        match status_map.get_mut(patch.uuid()) {
            Some(status) => {
                *status = value;
            }
            None => {
                status_map.insert(*patch.uuid(), value);
            }
        }

        Ok(())
    }
}

impl PatchManager {
    fn driver_get_patch_status(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<PatchStatus> {
        self.driver.status(patch)
    }

    fn driver_check_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.check(patch, flag)
    }

    fn driver_apply_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.driver.apply(patch)?;
        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn driver_remove_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.driver.remove(patch)?;
        self.set_patch_status(patch, PatchStatus::NotApplied)
    }

    fn driver_active_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.active(patch, flag)?;
        self.set_patch_status(patch, PatchStatus::Actived)
    }

    fn driver_deactive_patch(&mut self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.driver.deactive(patch, flag)?;
        self.set_patch_status(patch, PatchStatus::Deactived)
    }

    fn driver_accept_patch(&mut self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        self.set_patch_status(patch, PatchStatus::Accepted)
    }
}

impl Drop for PatchManager {
    fn drop(&mut self) {
        self.finallize()
    }
}
