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
use log::{debug, error};
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

type ElfPatchMap = Arc<Mutex<IndexMap<PathBuf, ElfPatchRecord>>>;

#[derive(Default)]
struct ElfPatchRecord {
    patch_map: IndexMap<Uuid, PathBuf>, // Patch applied to target elf (uuid and patch file)
    processes: IndexSet<i32>,           // Target elf process list
}

pub struct UserPatchDriver {
    patch_target_map: IndexMap<OsString, PatchTarget>,
    patch_status_map: IndexMap<Uuid, PatchStatus>,
    elf_patch_map: ElfPatchMap,
    patch_monitor: UserPatchMonitor,
}

impl UserPatchDriver {
    pub fn new() -> Result<Self> {
        let elf_patch_map = Arc::new(Mutex::new(IndexMap::new()));
        let patch_monitor = UserPatchMonitor::new(elf_patch_map.clone(), Self::patch_new_process)?;

        let instance = Self {
            patch_target_map: IndexMap::new(),
            patch_status_map: IndexMap::new(),
            elf_patch_map,
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
    fn patch_new_process(elf_patch_map: ElfPatchMap, target_elf: &Path) {
        let process_list = match process::find_target_process(target_elf) {
            Ok(processes) => processes,
            Err(_) => return,
        };

        let mut patch_map = elf_patch_map.lock();
        let patch_record = match patch_map.get_mut(target_elf) {
            Some(record) => record,
            None => return,
        };

        let need_active = process_list
            .difference(&patch_record.processes)
            .copied()
            .collect::<Vec<_>>();

        // Active patch
        for (uuid, patch_file) in &patch_record.patch_map {
            if !need_active.is_empty() {
                debug!(
                    "Upatch: Activating patch '{}' ({}) to process {:?}",
                    uuid,
                    target_elf.display(),
                    need_active,
                );
            }
            for pid in &need_active {
                if let Err(e) = sys::active_patch(uuid, *pid, target_elf, patch_file)
                    .with_context(|| format!("Failed to patch process, pid={}", pid))
                {
                    error!("{}", e);
                    continue;
                }
                patch_record.processes.insert(*pid);
            }
        }

        // Remove process no longer exists
        let need_remove = patch_record
            .processes
            .difference(&process_list)
            .copied()
            .collect::<Vec<_>>();
        for pid in need_remove {
            patch_record.processes.remove(&pid);
        }
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

        debug!(
            "Upatch: Applying patch '{}' ({})",
            patch_uuid,
            patch.patch_file.display()
        );
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

        debug!(
            "Upatch: Removing patch '{}' ({})",
            patch_uuid,
            patch.patch_file.display()
        );
        self.patch_status_map.remove(&patch_uuid);

        Ok(())
    }

    pub fn active(&mut self, patch: &UserPatch) -> Result<()> {
        let uuid = patch.uuid;
        let patch_status = self.get_patch_status(uuid)?;
        ensure!(
            patch_status == PatchStatus::Deactived,
            "Upatch: Invalid patch status"
        );

        let target_elf = patch.target_elf.as_path();
        let patch_file = patch.patch_file.as_path();
        let process_list = process::find_target_process(target_elf)?;

        let mut patch_map = self.elf_patch_map.lock();
        let patch_record = patch_map.entry(target_elf.to_path_buf()).or_default();

        let need_active = process_list
            .difference(&patch_record.processes)
            .copied()
            .collect::<Vec<_>>();
        let need_remove = patch_record
            .processes
            .difference(&process_list)
            .copied()
            .collect::<Vec<_>>();
        let mut need_start_watch = false;

        // Active patch
        if !need_active.is_empty() {
            debug!(
                "Upatch: Activating patch '{}' ({}) to process {:?}",
                uuid,
                target_elf.display(),
                need_active,
            );
        }
        for pid in need_active {
            sys::active_patch(&uuid, pid, target_elf, patch_file)
                .with_context(|| format!("Failed to patch process, pid={}", pid))?;
            patch_record.processes.insert(pid);
        }

        // Remove process no longer exists
        for pid in need_remove {
            patch_record.processes.remove(&pid);
        }

        // If elf is not patched before, start watching it & add a new entry
        if !patch_record.patch_map.contains_key(&uuid) {
            patch_record
                .patch_map
                .insert(uuid, patch_file.to_path_buf());
            need_start_watch = true;
        }

        drop(patch_map);

        if need_start_watch {
            self.patch_monitor.watch_file(target_elf)?;
        }
        self.set_patch_status(uuid, PatchStatus::Actived)?;
        self.add_patch_symbols(patch);

        Ok(())
    }

    pub fn deactive(&mut self, patch: &UserPatch) -> Result<()> {
        let uuid = patch.uuid;
        let patch_status = self.get_patch_status(uuid)?;
        ensure!(
            patch_status == PatchStatus::Actived,
            "Upatch: Invalid patch status"
        );

        let target_elf = patch.target_elf.as_path();
        let patch_file = patch.patch_file.as_path();
        let process_list = process::find_target_process(target_elf)?;

        let mut patch_map = self.elf_patch_map.lock();
        let patch_record = patch_map
            .get_mut(target_elf)
            .context("Failed to find elf patch record")?;

        let need_deactive = process_list
            .intersection(&patch_record.processes)
            .copied()
            .collect::<Vec<_>>();
        let need_removed = patch_record
            .processes
            .difference(&process_list)
            .copied()
            .collect::<Vec<_>>();
        let mut need_stop_watch = false;

        // Deactive patch
        if !need_deactive.is_empty() {
            debug!(
                "Upatch: Deactivating patch '{}' ({}) of process {:?}",
                uuid,
                target_elf.display(),
                need_deactive,
            );
        }
        for pid in need_deactive {
            sys::deactive_patch(&uuid, pid, target_elf, patch_file)
                .with_context(|| format!("Failed to unpatch process, pid={}", pid))?;
            patch_record.processes.remove(&pid); // remove process from record
        }

        // Remove process no longer exists
        for pid in need_removed {
            patch_record.processes.remove(&pid);
        }

        // Remove patch from elf patch record
        patch_record.patch_map.remove(&uuid);

        // If elf has no more patch, stop watching it & remove the entry
        if patch_record.patch_map.is_empty() {
            patch_map.remove(target_elf);
            need_stop_watch = true;
        }

        drop(patch_map);

        if need_stop_watch {
            self.patch_monitor.ignore_file(target_elf)?;
        }
        self.set_patch_status(uuid, PatchStatus::Deactived)?;
        self.remove_patch_symbols(patch);

        Ok(())
    }
}
