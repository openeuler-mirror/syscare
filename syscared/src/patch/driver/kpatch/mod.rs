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
    ffi::OsString,
    fmt::Write,
    iter::FromIterator,
};

use anyhow::{anyhow, ensure, Context, Result};
use log::debug;

use syscare_abi::PatchStatus;
use syscare_common::{
    concat_os,
    os::{self, kernel, selinux},
    util::digest,
};

use crate::{config::KernelPatchConfig, patch::entity::KernelPatch};

mod sys;
mod target;

use target::PatchTarget;

pub struct KernelPatchDriver {
    target_map: HashMap<OsString, PatchTarget>, // object name -> object
    blocked_targets: HashSet<OsString>,
}

impl KernelPatchDriver {
    pub fn new(config: &KernelPatchConfig) -> Result<Self> {
        Ok(Self {
            target_map: HashMap::new(),
            blocked_targets: HashSet::from_iter(config.blocked.iter().cloned()),
        })
    }
}

impl KernelPatchDriver {
    fn register_patch(&mut self, patch: &KernelPatch) {
        for object_name in patch.functions.keys() {
            self.target_map
                .entry(object_name.clone())
                .or_insert_with(|| PatchTarget::new(object_name.clone()))
                .add_patch(patch);
        }
    }

    fn unregister_patch(&mut self, patch: &KernelPatch) {
        self.target_map.retain(|_, object| {
            object.remove_patch(patch);
            object.is_patched()
        });
    }
}

impl KernelPatchDriver {
    fn check_consistency(patch: &KernelPatch) -> Result<()> {
        let real_checksum = digest::file(&patch.patch_file)?;
        debug!("Target checksum: '{}'", patch.checksum);
        debug!("Expected checksum: '{}'", real_checksum);

        ensure!(
            patch.checksum == real_checksum,
            "Kpatch: Patch consistency check failed",
        );
        Ok(())
    }

    fn check_compatiblity(patch: &KernelPatch) -> Result<()> {
        const KERNEL_NAME_PREFIX: &str = "kernel-";

        let patch_target = patch.pkg_name.as_str();
        if !patch_target.starts_with(KERNEL_NAME_PREFIX) {
            return Ok(());
        }

        let current_kernel = concat_os!(KERNEL_NAME_PREFIX, os::kernel::version());
        debug!("Patch target:   '{}'", patch_target);
        debug!("Current kernel: '{}'", current_kernel.to_string_lossy());

        ensure!(
            current_kernel == patch_target,
            "Kpatch: Patch is incompatible",
        );
        Ok(())
    }

    fn check_dependency(patch: &KernelPatch) -> Result<()> {
        const VMLINUX_MODULE_NAME: &str = "vmlinux";

        let depend_modules = patch.functions.keys().cloned().collect::<HashSet<_>>();
        let inserted_modules =
            kernel::list_modules().context("Kpatch: Failed to list kernel modules")?;
        let needed_modules = depend_modules
            .difference(&inserted_modules)
            .filter(|&module_name| module_name != VMLINUX_MODULE_NAME)
            .collect::<Vec<_>>();

        ensure!(needed_modules.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Kpatch: Patch target does not exist")?;
            for name in needed_modules {
                writeln!(msg, "* Module '{}'", name.to_string_lossy())?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }

    pub fn check_conflicted_patches(&self, patch: &KernelPatch) -> Result<()> {
        let conflicted: HashSet<_> = self
            .target_map
            .values()
            .flat_map(|object| object.get_conflicted_patches(patch))
            .collect();

        ensure!(conflicted.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Kpatch: Patch is conflicted with")?;
            for uuid in conflicted {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }

    pub fn check_overridden_patches(&self, patch: &KernelPatch) -> Result<()> {
        let overridden: HashSet<_> = self
            .target_map
            .values()
            .flat_map(|object| object.get_overridden_patches(patch))
            .collect();

        ensure!(overridden.is_empty(), {
            let mut msg = String::new();
            writeln!(msg, "Kpatch: Patch is overridden by")?;
            for uuid in overridden {
                writeln!(msg, "* Patch '{}'", uuid)?;
            }
            msg.pop();
            msg
        });
        Ok(())
    }
}

impl KernelPatchDriver {
    pub fn check_patch(&self, patch: &KernelPatch) -> Result<()> {
        Self::check_consistency(patch)?;
        Self::check_compatiblity(patch)?;
        Self::check_dependency(patch)?;
        Ok(())
    }

    pub fn get_patch_status(&self, patch: &KernelPatch) -> Result<PatchStatus> {
        sys::get_patch_status(&patch.status_file).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to get patch status, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn load_patch(&mut self, patch: &KernelPatch) -> Result<()> {
        ensure!(
            !self.blocked_targets.contains(&patch.target_name),
            "Kpatch: Patch target '{}' is blocked",
            patch.target_name.to_string_lossy(),
        );

        if selinux::get_status() == selinux::Status::Enforcing {
            kernel::relable_module_file(&patch.patch_file).map_err(|e| {
                anyhow!(
                    "Kpatch: Failed to relable patch file, {}",
                    e.to_string().to_lowercase()
                )
            })?;
        }
        sys::load_patch(&patch.patch_file).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to load patch, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn remove_patch(&mut self, patch: &KernelPatch) -> Result<()> {
        sys::remove_patch(&patch.module.name).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to remove patch, {}",
                e.to_string().to_lowercase()
            )
        })
    }

    pub fn active_patch(&mut self, patch: &KernelPatch) -> Result<()> {
        sys::active_patch(&patch.status_file).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to active patch, {}",
                e.to_string().to_lowercase()
            )
        })?;
        self.register_patch(patch);

        Ok(())
    }

    pub fn deactive_patch(&mut self, patch: &KernelPatch) -> Result<()> {
        sys::deactive_patch(&patch.status_file).map_err(|e| {
            anyhow!(
                "Kpatch: Failed to deactive patch, {}",
                e.to_string().to_lowercase()
            )
        })?;
        self.unregister_patch(patch);

        Ok(())
    }
}
