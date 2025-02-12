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
    ffi::{OsStr, OsString},
    fmt::Write,
    iter::FromIterator,
};

use anyhow::{ensure, Result};
use indexmap::{indexset, IndexMap, IndexSet};
use log::{debug, info};

use syscare_abi::PatchStatus;
use syscare_common::{concat_os, os, util::digest};

use crate::{
    config::KernelPatchConfig,
    patch::entity::{KernelPatch, KernelPatchFunction},
};

mod sys;
mod target;

use target::PatchTarget;

pub struct KernelPatchDriver {
    target_map: IndexMap<OsString, PatchTarget>,
    blocked_targets: IndexSet<OsString>,
}

impl KernelPatchDriver {
    pub fn new(config: &KernelPatchConfig) -> Result<Self> {
        Ok(Self {
            target_map: IndexMap::new(),
            blocked_targets: IndexSet::from_iter(config.blocked.iter().cloned()),
        })
    }
}

impl KernelPatchDriver {
    fn group_patch_targets(patch: &KernelPatch) -> IndexSet<&OsStr> {
        let mut patch_targets = IndexSet::new();

        for function in &patch.functions {
            patch_targets.insert(function.object.as_os_str());
        }
        patch_targets
    }

    pub fn group_patch_functions(
        patch: &KernelPatch,
    ) -> IndexMap<&OsStr, Vec<&KernelPatchFunction>> {
        let mut patch_function_map: IndexMap<&OsStr, Vec<&KernelPatchFunction>> = IndexMap::new();

        for function in &patch.functions {
            patch_function_map
                .entry(function.object.as_os_str())
                .or_default()
                .push(function);
        }
        patch_function_map
    }
}

impl KernelPatchDriver {
    fn add_patch_target(&mut self, patch: &KernelPatch) {
        for target_name in Self::group_patch_targets(patch) {
            if !self.target_map.contains_key(target_name) {
                self.target_map.insert(
                    target_name.to_os_string(),
                    PatchTarget::new(target_name.to_os_string()),
                );
            }
        }
    }

    fn remove_patch_target(&mut self, patch: &KernelPatch) {
        for target_name in Self::group_patch_targets(patch) {
            if let Some(target) = self.target_map.get_mut(target_name) {
                if !target.has_function() {
                    self.target_map.remove(target_name);
                }
            }
        }
    }

    fn add_patch_functions(&mut self, patch: &KernelPatch) {
        for (target_name, functions) in Self::group_patch_functions(patch) {
            if let Some(target) = self.target_map.get_mut(target_name) {
                target.add_functions(patch.uuid, functions);
            }
        }
    }

    fn remove_patch_functions(&mut self, patch: &KernelPatch) {
        for (target_name, functions) in Self::group_patch_functions(patch) {
            if let Some(target) = self.target_map.get_mut(target_name) {
                target.remove_functions(&patch.uuid, functions);
            }
        }
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
        let current_kernel = concat_os!(KERNEL_NAME_PREFIX, os::kernel::version());
        debug!("Patch target:   '{}'", patch_target);
        debug!("Current kernel: '{}'", current_kernel.to_string_lossy());

        if !patch_target.starts_with("KERNEL_NAME_PREFIX") {
            return Ok(());
        }
        ensure!(
            current_kernel == patch_target,
            "Kpatch: Patch is incompatible",
        );
        Ok(())
    }

    fn check_dependency(patch: &KernelPatch) -> Result<()> {
        const VMLINUX_MODULE_NAME: &str = "vmlinux";

        let mut non_exist_kmod = IndexSet::new();

        let kmod_list = sys::list_kernel_modules()?;
        for kmod_name in Self::group_patch_targets(patch) {
            if kmod_name == VMLINUX_MODULE_NAME {
                continue;
            }
            if kmod_list.iter().any(|name| name == kmod_name) {
                continue;
            }
            non_exist_kmod.insert(kmod_name);
        }

        ensure!(non_exist_kmod.is_empty(), {
            let mut err_msg = String::new();

            writeln!(&mut err_msg, "Kpatch: Patch target does not exist")?;
            for kmod_name in non_exist_kmod {
                writeln!(&mut err_msg, "* Module '{}'", kmod_name.to_string_lossy())?;
            }
            err_msg.pop();

            err_msg
        });
        Ok(())
    }

    pub fn check_conflict_functions(&self, patch: &KernelPatch) -> Result<()> {
        let mut conflict_patches = indexset! {};

        let target_functions = Self::group_patch_functions(patch);
        for (target_name, functions) in target_functions {
            if let Some(target) = self.target_map.get(target_name) {
                conflict_patches.extend(
                    target
                        .get_conflicts(functions)
                        .into_iter()
                        .map(|record| record.uuid),
                );
            }
        }

        ensure!(conflict_patches.is_empty(), {
            let mut err_msg = String::new();

            writeln!(&mut err_msg, "Kpatch: Patch is conflicted with")?;
            for uuid in conflict_patches.into_iter() {
                writeln!(&mut err_msg, "* Patch '{}'", uuid)?;
            }
            err_msg.pop();

            err_msg
        });
        Ok(())
    }

    pub fn check_override_functions(&self, patch: &KernelPatch) -> Result<()> {
        let mut override_patches = indexset! {};

        let target_functions = Self::group_patch_functions(patch);
        for (target_name, functions) in target_functions {
            if let Some(target) = self.target_map.get(target_name) {
                override_patches.extend(
                    target
                        .get_overrides(&patch.uuid, functions)
                        .into_iter()
                        .map(|record| record.uuid),
                );
            }
        }

        ensure!(override_patches.is_empty(), {
            let mut err_msg = String::new();

            writeln!(&mut err_msg, "Kpatch: Patch is overrided by")?;
            for uuid in override_patches.into_iter() {
                writeln!(&mut err_msg, "* Patch '{}'", uuid)?;
            }
            err_msg.pop();

            err_msg
        });
        Ok(())
    }
}

impl KernelPatchDriver {
    pub fn status(&self, patch: &KernelPatch) -> Result<PatchStatus> {
        sys::read_patch_status(patch)
    }

    pub fn check(&self, patch: &KernelPatch) -> Result<()> {
        Self::check_consistency(patch)?;
        Self::check_compatiblity(patch)?;
        Self::check_dependency(patch)?;

        Ok(())
    }

    pub fn apply(&mut self, patch: &KernelPatch) -> Result<()> {
        info!(
            "Applying patch '{}' ({})",
            patch.uuid,
            patch.patch_file.display()
        );

        ensure!(
            self.blocked_targets.contains(&patch.target_name),
            "Patch target '{}' is blocked",
            patch.target_name.to_string_lossy(),
        );
        sys::selinux_relable_patch(patch)?;
        sys::apply_patch(patch)?;
        self.add_patch_target(patch);

        Ok(())
    }

    pub fn remove(&mut self, patch: &KernelPatch) -> Result<()> {
        info!(
            "Removing patch '{}' ({})",
            patch.uuid,
            patch.patch_file.display()
        );
        sys::remove_patch(patch)?;
        self.remove_patch_target(patch);

        Ok(())
    }

    pub fn active(&mut self, patch: &KernelPatch) -> Result<()> {
        info!(
            "Activating patch '{}' ({})",
            patch.uuid,
            patch.patch_file.display()
        );
        sys::active_patch(patch)?;
        self.add_patch_functions(patch);

        Ok(())
    }

    pub fn deactive(&mut self, patch: &KernelPatch) -> Result<()> {
        info!(
            "Deactivating patch '{}' ({})",
            patch.uuid,
            patch.patch_file.display()
        );
        sys::deactive_patch(patch)?;
        self.remove_patch_functions(patch);

        Ok(())
    }
}
