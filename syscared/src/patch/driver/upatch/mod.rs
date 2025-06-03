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
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::Write,
    iter::FromIterator,
    os::linux::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, ensure, Result};
use log::{debug, warn};
use parking_lot::RwLock;
use uuid::Uuid;

use syscare_abi::PatchStatus;
use syscare_common::{fs, util::digest};

use crate::{config::UserPatchConfig, patch::entity::UserPatch};

mod monitor;
mod sys;
mod target;

use monitor::UserPatchMonitor;
use target::PatchTarget;

pub struct UserPatchDriver {
    status_map: HashMap<Uuid, PatchStatus>,
    target_map: Arc<RwLock<HashMap<PathBuf, PatchTarget>>>,
    skipped_files: Arc<HashSet<PathBuf>>,
    monitor: UserPatchMonitor,
}

impl UserPatchDriver {
    pub fn new(config: &UserPatchConfig) -> Result<Self> {
        let target_map = Arc::new(RwLock::new(HashMap::new()));
        let skipped_files = Arc::new(HashSet::from_iter(config.skipped.iter().cloned()));

        Ok(Self {
            status_map: HashMap::new(),
            target_map: target_map.clone(),
            skipped_files: skipped_files.clone(),
            monitor: UserPatchMonitor::new(move |target_elfs| {
                for target_elf in target_elfs {
                    Self::patch_new_process(&target_map, &skipped_files, target_elf);
                }
            })?,
        })
    }
}

impl UserPatchDriver {
    #[inline]
    fn read_patch_status(&self, uuid: &Uuid) -> PatchStatus {
        self.status_map
            .get(uuid)
            .copied()
            .unwrap_or(PatchStatus::NotApplied)
    }

    #[inline]
    fn write_patch_status(&mut self, uuid: &Uuid, value: PatchStatus) {
        *self.status_map.entry(*uuid).or_default() = value;
    }

    fn remove_patch_status(&mut self, uuid: &Uuid) {
        self.status_map.remove(uuid);
    }
}

impl UserPatchDriver {
    fn check_consistency(patch: &UserPatch) -> Result<()> {
        let real_checksum = digest::file(&patch.patch_file)?;
        debug!("Target checksum: '{}'", patch.checksum);
        debug!("Expected checksum: '{}'", real_checksum);

        ensure!(
            patch.checksum == real_checksum,
            "Upatch: Patch consistency check failed",
        );
        Ok(())
    }

    pub fn check_conflicted_patches(&self, patch: &UserPatch) -> Result<()> {
        let conflicted = match self.target_map.read().get(&patch.target_elf) {
            Some(target) => target.get_conflicted_patches(patch).collect(),
            None => HashSet::new(),
        };

        ensure!(conflicted.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Patch is conflicted with")?;
            for uuid in conflicted.into_iter() {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }

    pub fn check_overridden_patches(&self, patch: &UserPatch) -> Result<()> {
        let overridden = match self.target_map.read().get(&patch.target_elf) {
            Some(target) => target.get_overridden_patches(patch).collect(),
            None => HashSet::new(),
        };

        ensure!(overridden.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Patch is overridden by")?;
            for uuid in overridden.into_iter() {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }
}

impl UserPatchDriver {
    #[inline]
    fn parse_process_id(proc_path: &Path) -> Option<i32> {
        proc_path
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::parse)
            .and_then(Result::ok)
    }

    fn find_target_process<P: AsRef<Path>>(
        skipped_files: &HashSet<PathBuf>,
        target_elf: P,
    ) -> Result<HashSet<i32>> {
        let mut target_pids = HashSet::new();
        let target_path = target_elf.as_ref();
        let target_inode = target_path.metadata()?.st_ino();

        for proc_path in fs::list_dirs("/proc", fs::TraverseOptions { recursive: false })? {
            let pid = match Self::parse_process_id(&proc_path) {
                Some(pid) => pid,
                None => continue,
            };
            let exec_path = match fs::read_link(format!("/proc/{}/exe", pid)) {
                Ok(file_path) => file_path,
                Err(_) => continue,
            };
            if skipped_files.contains(&exec_path) {
                continue;
            }
            // Try to match binary path
            if exec_path == target_path {
                target_pids.insert(pid);
                continue;
            }
            // Try to match mapped files
            let map_files = fs::list_symlinks(
                format!("/proc/{}/map_files", pid),
                fs::TraverseOptions { recursive: false },
            )?;
            for mapped_file in map_files {
                if let Ok(mapped_inode) = mapped_file
                    .read_link()
                    .and_then(|file_path| Ok(file_path.metadata()?.st_ino()))
                {
                    if mapped_inode == target_inode {
                        target_pids.insert(pid);
                        break;
                    }
                };
            }
        }

        Ok(target_pids)
    }

    fn patch_new_process(
        target_map: &RwLock<HashMap<PathBuf, PatchTarget>>,
        skipped_files: &HashSet<PathBuf>,
        target_elf: &Path,
    ) {
        let process_list = match Self::find_target_process(skipped_files, target_elf) {
            Ok(pids) => pids,
            Err(_) => return,
        };

        let mut target_map = target_map.write();
        let patch_target = match target_map.get_mut(target_elf) {
            Some(target) => target,
            None => return,
        };
        patch_target.clean_dead_process(&process_list);

        let all_patches = patch_target.all_patches().collect::<Vec<_>>();
        let need_actived = patch_target.need_actived(&process_list);

        for (uuid, patch_file) in all_patches {
            if !need_actived.is_empty() {
                debug!(
                    "Upatch: Activating patch '{}' ({}) for process {:?}",
                    uuid,
                    target_elf.display(),
                    need_actived,
                );
            }
            for &pid in &need_actived {
                match sys::active_patch(&uuid, pid, target_elf, &patch_file) {
                    Ok(_) => patch_target.add_process(pid),
                    Err(e) => {
                        warn!(
                            "Upatch: Failed to active patch '{}' for process {}, {}",
                            uuid,
                            pid,
                            e.to_string().to_lowercase(),
                        );
                    }
                }
            }
        }
    }
}

impl UserPatchDriver {
    pub fn check_patch(&self, patch: &UserPatch) -> Result<()> {
        Self::check_consistency(patch)?;
        Ok(())
    }

    pub fn get_patch_status(&self, patch: &UserPatch) -> Result<PatchStatus> {
        Ok(self.read_patch_status(&patch.uuid))
    }

    pub fn load_patch(&mut self, patch: &UserPatch) -> Result<()> {
        self.write_patch_status(&patch.uuid, PatchStatus::Deactived);
        Ok(())
    }

    pub fn remove_patch(&mut self, patch: &UserPatch) -> Result<()> {
        self.remove_patch_status(&patch.uuid);
        Ok(())
    }

    pub fn active_patch(&mut self, patch: &UserPatch) -> Result<()> {
        let process_list = Self::find_target_process(&self.skipped_files, &patch.target_elf)?;

        let mut target_map = self.target_map.write();
        let patch_target = target_map.entry(patch.target_elf.clone()).or_default();
        patch_target.clean_dead_process(&process_list);

        // If target is not patched before, start watching it
        let start_watch = !patch_target.is_patched();

        // Active patch
        let need_actived = patch_target.need_actived(&process_list);

        let mut results = Vec::new();
        for pid in need_actived {
            let result = sys::active_patch(&patch.uuid, pid, &patch.target_elf, &patch.patch_file);
            results.push((pid, result));
        }

        // Return error if all process fails
        if !results.is_empty() && results.iter().all(|(_, result)| result.is_err()) {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Failed to active patch")?;
            for (pid, result) in &results {
                if let Err(e) = result {
                    writeln!(msg, "* Process {}: {}", pid, e)?;
                }
            }
            msg.pop();
            bail!(msg);
        }

        // Process results
        for (pid, result) in results {
            match result {
                Ok(_) => patch_target.add_process(pid),
                Err(e) => {
                    warn!(
                        "Upatch: Failed to active patch '{}' for process {}, {}",
                        patch.uuid,
                        pid,
                        e.to_string().to_lowercase(),
                    );
                }
            }
        }
        patch_target.add_patch(patch);

        // Drop the lock
        drop(target_map);

        if start_watch {
            self.monitor.watch_file(&patch.target_elf)?;
        }

        self.write_patch_status(&patch.uuid, PatchStatus::Actived);
        Ok(())
    }

    pub fn deactive_patch(&mut self, patch: &UserPatch) -> Result<()> {
        let process_list = Self::find_target_process(&self.skipped_files, &patch.target_elf)?;

        let mut target_map = self.target_map.write();
        let patch_target = target_map.entry(patch.target_elf.clone()).or_default();
        patch_target.clean_dead_process(&process_list);

        // Deactive patch
        let need_deactive = patch_target.need_deactived(&process_list);

        let mut results = Vec::new();
        for pid in need_deactive {
            let result =
                sys::deactive_patch(&patch.uuid, pid, &patch.target_elf, &patch.patch_file);
            results.push((pid, result));
        }

        // Return error if all process fails
        if !results.is_empty() && results.iter().any(|(_, result)| result.is_err()) {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Failed to deactive patch")?;
            for (pid, result) in &results {
                if let Err(e) = result {
                    writeln!(msg, "* Process {}: {}", pid, e)?;
                }
            }
            msg.pop();
            bail!(msg);
        }

        // Process results
        for (pid, result) in results {
            match result {
                Ok(_) => patch_target.remove_process(pid),
                Err(e) => {
                    warn!(
                        "Upatch: Failed to deactive patch '{}' for process {}, {}",
                        patch.uuid,
                        pid,
                        e.to_string().to_lowercase(),
                    );
                }
            }
        }
        patch_target.remove_patch(patch);

        // If target is no longer has patch, stop watching it
        let stop_watch = !patch_target.is_patched();

        drop(target_map);

        if stop_watch {
            self.monitor.ignore_file(&patch.target_elf)?;
        }

        self.write_patch_status(&patch.uuid, PatchStatus::Deactived);
        Ok(())
    }
}
