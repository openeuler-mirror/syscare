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
    fmt::Write,
    fs::File,
    iter::FromIterator,
    os::unix::io::{AsRawFd, RawFd},
    path::PathBuf,
};

use anyhow::{anyhow, ensure, Result};
use log::debug;

use syscare_abi::PatchStatus;
use syscare_common::{
    os::{kernel, selinux},
    util::digest,
};

use crate::{config::UserPatchConfig, patch::entity::UserPatch};

mod sys;
mod target;

use target::PatchTarget;

pub struct UserPatchDriver {
    ioctl_dev: File,
    target_map: HashMap<PathBuf, PatchTarget>, // target elf -> target
    blocked_files: HashSet<PathBuf>,
    _guard: kernel::ModuleGuard,
}

impl UserPatchDriver {
    pub fn new(config: &UserPatchConfig) -> Result<Self> {
        const UPATCH_KMOD_FILE: &str = "/usr/libexec/syscare/upatch_manage.ko";
        const UPATCH_DEV_FILE: &str = "/dev/upatch_manage";

        if selinux::get_status() == selinux::Status::Enforcing {
            kernel::relable_module_file(UPATCH_KMOD_FILE).map_err(|e| {
                anyhow!(
                    "Failed to relable upatch kernel module, {}",
                    e.to_string().to_lowercase()
                )
            })?;
        }
        let guard = kernel::insert_module_guarded(UPATCH_KMOD_FILE).map_err(|e| {
            anyhow!(
                "Failed to insert upatch kernel module, {}",
                e.to_string().to_lowercase()
            )
        })?;

        let driver = Self {
            ioctl_dev: File::open(UPATCH_DEV_FILE).map_err(|e| {
                anyhow!(
                    "Failed to open device '{}', {}",
                    UPATCH_DEV_FILE,
                    e.to_string().to_lowercase()
                )
            })?,
            target_map: HashMap::new(),
            blocked_files: HashSet::from_iter(config.blocked.iter().cloned()),
            _guard: guard,
        };
        Ok(driver)
    }
}

impl UserPatchDriver {
    fn register_patch(&mut self, patch: &UserPatch) {
        self.target_map
            .entry(patch.target_elf.clone())
            .or_default()
            .add_patch(patch);
    }

    fn unregister_patch(&mut self, patch: &UserPatch) {
        let target = match self.target_map.get_mut(&patch.target_elf) {
            Some(target) => target,
            None => return,
        };
        target.remove_patch(patch);

        if !target.is_patched() {
            self.target_map.remove(&patch.target_elf);
        }
    }
}

impl UserPatchDriver {
    #[inline]
    fn ioctl_fd(&self) -> RawFd {
        self.ioctl_dev.as_raw_fd()
    }

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
        let conflicted = match self.target_map.get(&patch.target_elf) {
            Some(target) => target.get_conflicted_patches(patch).collect(),
            None => HashSet::new(),
        };

        ensure!(conflicted.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Patch is conflicted with")?;
            for uuid in conflicted {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }

    pub fn check_overridden_patches(&self, patch: &UserPatch) -> Result<()> {
        let overridden = match self.target_map.get(&patch.target_elf) {
            Some(target) => target.get_overridden_patches(patch).collect(),
            None => HashSet::new(),
        };

        ensure!(overridden.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Upatch: Patch is overridden by")?;
            for uuid in overridden {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }
}

impl UserPatchDriver {
    pub fn check_patch(&self, patch: &UserPatch) -> Result<()> {
        Self::check_consistency(patch)?;
        Ok(())
    }

    pub fn get_patch_status(&self, patch: &UserPatch) -> Result<PatchStatus> {
        sys::get_patch_status(self.ioctl_fd(), &patch.patch_file).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to get patch status, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn load_patch(&mut self, patch: &UserPatch) -> Result<()> {
        ensure!(
            !self.blocked_files.contains(&patch.target_elf),
            "Upatch: Patch target '{}' is blocked",
            patch.target_elf.display(),
        );
        sys::load_patch(self.ioctl_fd(), &patch.patch_file, &patch.target_elf).map_err(|e| {
            anyhow!(
                "Upatch: Failed to load patch, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn remove_patch(&mut self, patch: &UserPatch) -> Result<()> {
        sys::remove_patch(self.ioctl_fd(), &patch.patch_file).map_err(|e| {
            anyhow!(
                "Upatch: Failed to remove patch, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn active_patch(&mut self, patch: &UserPatch) -> Result<()> {
        sys::active_patch(self.ioctl_fd(), &patch.patch_file).map_err(|e| {
            anyhow!(
                "Upatch: Failed to active patch, {}",
                e.to_string().to_lowercase()
            )
        })?;
        self.register_patch(patch);

        Ok(())
    }

    pub fn deactive_patch(&mut self, patch: &UserPatch) -> Result<()> {
        sys::deactive_patch(self.ioctl_fd(), &patch.patch_file).map_err(|e| {
            anyhow!(
                "Upatch: Failed to deactive patch, {}",
                e.to_string().to_lowercase()
            )
        })?;
        self.unregister_patch(patch);

        Ok(())
    }
}
