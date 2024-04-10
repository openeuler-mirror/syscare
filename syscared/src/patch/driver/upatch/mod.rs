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
    ffi::OsString,
    fmt::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{ensure, Context, Result};
use indexmap::{indexset, IndexMap, IndexSet};
use log::{debug, info};
use parking_lot::Mutex;
use uuid::Uuid;

use syscare_abi::PatchStatus;
use syscare_common::util::digest;

use crate::patch::entity::UserPatch;

mod monitor;
mod process;
mod sys;
mod target;

use monitor::UserPatchMonitor;
use target::PatchTarget;

type ActivePatchMap = Arc<Mutex<IndexMap<PathBuf, ActivePatch>>>;

#[derive(Default)]
struct ActivePatch {
    patch_list: Vec<(Uuid, PathBuf)>, // Patch applied to target elf (uuid and patch file)
    process_list: IndexSet<i32>,      // Target elf process list
}

pub struct UserPatchDriver {
    patch_target_map: IndexMap<OsString, PatchTarget>,
    patch_status_map: IndexMap<Uuid, PatchStatus>,
    active_patch_map: ActivePatchMap,
    patch_monitor: UserPatchMonitor,
}

impl UserPatchDriver {
    pub fn new() -> Result<Self> {
        let active_patch_map = Arc::new(Mutex::new(IndexMap::new()));
        let patch_monitor =
            UserPatchMonitor::new(active_patch_map.clone(), Self::on_new_process_created)?;

        let instance = Self {
            patch_target_map: IndexMap::new(),
            patch_status_map: IndexMap::new(),
            active_patch_map,
            patch_monitor,
        };
        Ok(instance)
    }
}

impl UserPatchDriver {
    fn check_consistency(patch: &UserPatch) -> Result<()> {
        let patch_file = patch.patch_file.as_path();
        let real_checksum = digest::file(patch_file)?;
        debug!("Target checksum: '{}'", patch.checksum);
        debug!("Expected checksum: '{}'", real_checksum);

        ensure!(
            patch.checksum == real_checksum,
            "Upatch: Patch consistency check failed",
        );
        Ok(())
    }

    fn check_compatiblity(_patch: &UserPatch) -> Result<()> {
        Ok(())
    }

    pub fn check_conflict_symbols(&self, patch: &UserPatch) -> Result<()> {
        let patch_symbols = patch.symbols.as_slice();
        let target_name = patch.target_elf.as_os_str();
        let conflict_patches = match self.patch_target_map.get(target_name) {
            Some(target) => target
                .get_conflicts(patch_symbols)
                .into_iter()
                .map(|record| record.uuid)
                .collect(),
            None => indexset! {},
        };

        ensure!(conflict_patches.is_empty(), {
            let mut err_msg = String::new();

            writeln!(&mut err_msg, "Upatch: Patch is conflicted with")?;
            for uuid in conflict_patches.into_iter() {
                writeln!(&mut err_msg, "* Patch '{}'", uuid)?;
            }
            err_msg.pop();

            err_msg
        });
        Ok(())
    }

    pub fn check_override_symbols(&self, patch: &UserPatch) -> Result<()> {
        let patch_uuid = patch.uuid;
        let patch_symbols = patch.symbols.as_slice();
        let target_name = patch.target_elf.as_os_str();
        let override_patches = match self.patch_target_map.get(target_name) {
            Some(target) => target
                .get_overrides(&patch_uuid, patch_symbols)
                .into_iter()
                .map(|record| record.uuid)
                .collect(),
            None => indexset! {},
        };

        ensure!(override_patches.is_empty(), {
            let mut err_msg = String::new();

            writeln!(&mut err_msg, "Upatch: Patch is overrided by")?;
            for uuid in override_patches.into_iter() {
                writeln!(&mut err_msg, "* Patch '{}'", uuid)?;
            }
            err_msg.pop();

            err_msg
        });

        Ok(())
    }
}

impl UserPatchDriver {
    fn add_patch_symbols(&mut self, patch: &UserPatch) {
        let target_name = patch.target_elf.as_os_str();

        let patch_uuid = patch.uuid;
        let patch_target = self
            .patch_target_map
            .entry(target_name.to_os_string())
            .or_insert_with(PatchTarget::new);
        let patch_symbols = patch.symbols.as_slice();

        patch_target.add_symbols(patch_uuid, patch_symbols);
    }

    fn remove_patch_symbols(&mut self, patch: &UserPatch) {
        let patch_uuid = patch.uuid;
        let patch_symbols = patch.symbols.as_slice();
        let target_name = patch.target_elf.as_os_str();

        if let Some(patch_target) = self.patch_target_map.get_mut(target_name) {
            patch_target.remove_symbols(&patch_uuid, patch_symbols);
        }
    }
}

impl UserPatchDriver {
    fn on_new_process_created(active_patch_map: ActivePatchMap, target_elf: &Path) -> Result<()> {
        // find actived patch
        if let Some(patch_record) = active_patch_map.lock().get_mut(target_elf) {
            let current_process_list = process::find_target_process(target_elf)?;
            let patched_process_list = &patch_record.process_list;

            // Filter patched pid
            let pid_list = current_process_list
                .iter()
                .filter(|pid| !patched_process_list.contains(*pid))
                .copied()
                .collect::<Vec<_>>();
            if pid_list.is_empty() {
                return Ok(());
            }

            for (uuid, patch_file) in &patch_record.patch_list {
                info!(
                    "Patching '{}' ({}) to process {:?}",
                    uuid,
                    target_elf.display(),
                    pid_list,
                );
                sys::active_patch(uuid, target_elf, patch_file, &pid_list)?;
            }

            patch_record.process_list = current_process_list;
        }

        Ok(())
    }
}

impl UserPatchDriver {
    #[inline]
    fn get_patch_status(&self, uuid: Uuid) -> Result<PatchStatus> {
        let patch_status = self
            .patch_status_map
            .get(&uuid)
            .copied()
            .context("Upatch: Patch does not exist")?;

        Ok(patch_status)
    }

    #[inline]
    fn set_patch_status(&mut self, uuid: Uuid, value: PatchStatus) -> Result<()> {
        let patch_status = self
            .patch_status_map
            .get_mut(&uuid)
            .context("Upatch: Patch does not exist")?;

        *patch_status = value;
        Ok(())
    }
}

impl UserPatchDriver {
    pub fn status(&self, patch: &UserPatch) -> Result<PatchStatus> {
        Ok(self
            .get_patch_status(patch.uuid)
            .unwrap_or(PatchStatus::NotApplied))
    }

    pub fn check(&self, patch: &UserPatch) -> Result<()> {
        Self::check_consistency(patch)?;
        Self::check_compatiblity(patch)?;

        Ok(())
    }

    pub fn apply(&mut self, patch: &UserPatch) -> Result<()> {
        let patch_uuid = patch.uuid;
        ensure!(
            self.get_patch_status(patch_uuid).is_err(),
            "Upatch: Patch already exists"
        );

        debug!("Upatch: Applying patch '{}'", patch_uuid);
        self.patch_status_map
            .insert(patch_uuid, PatchStatus::Deactived);

        Ok(())
    }

    pub fn remove(&mut self, patch: &UserPatch) -> Result<()> {
        let patch_uuid = patch.uuid;
        let patch_status = self.get_patch_status(patch_uuid)?;
        ensure!(
            patch_status == PatchStatus::Deactived,
            "Upatch: Invalid patch status"
        );

        debug!("Upatch: Removing patch '{}'", patch_uuid);
        self.patch_status_map.remove(&patch_uuid);

        Ok(())
    }

    pub fn active(&mut self, patch: &UserPatch) -> Result<()> {
        let patch_uuid = patch.uuid;
        let patch_status = self.get_patch_status(patch_uuid)?;
        ensure!(
            patch_status == PatchStatus::Deactived,
            "Upatch: Invalid patch status"
        );

        let target_elf = patch.target_elf.as_path();
        let patch_file = patch.patch_file.as_path();
        let pid_list = process::find_target_process(target_elf)?;
        sys::active_patch(&patch_uuid, target_elf, patch_file, &pid_list)?;

        let mut active_patch_map = self.active_patch_map.lock();
        let active_patch = active_patch_map
            .entry(target_elf.to_path_buf())
            .or_default();
        let patch_list = &mut active_patch.patch_list;

        patch_list.push((patch_uuid, patch_file.to_path_buf()));
        self.patch_monitor.watch_file(target_elf)?;

        drop(active_patch_map);

        self.set_patch_status(patch_uuid, PatchStatus::Actived)?;
        self.add_patch_symbols(patch);

        Ok(())
    }

    pub fn deactive(&mut self, patch: &UserPatch) -> Result<()> {
        let patch_uuid = patch.uuid;
        let patch_status = self.get_patch_status(patch_uuid)?;
        ensure!(
            patch_status == PatchStatus::Actived,
            "Upatch: Invalid patch status"
        );

        let target_elf = patch.target_elf.as_path();
        let patch_file = patch.patch_file.as_path();
        let pid_list = process::find_target_process(target_elf)?;
        sys::deactive_patch(&patch_uuid, target_elf, patch_file, &pid_list)?;

        let mut active_patch_map = self.active_patch_map.lock();
        let active_patch = active_patch_map
            .entry(target_elf.to_path_buf())
            .or_default();
        let patch_list = &mut active_patch.patch_list;

        patch_list.pop();
        if patch_list.is_empty() {
            self.patch_monitor.ignore_file(target_elf)?;
            active_patch_map.remove(target_elf);
        }

        drop(active_patch_map);

        self.set_patch_status(patch_uuid, PatchStatus::Deactived)?;
        self.remove_patch_symbols(patch);

        Ok(())
    }
}
