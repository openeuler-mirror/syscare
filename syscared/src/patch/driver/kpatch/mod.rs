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
};

use anyhow::{ensure, Context, Result};
use indexmap::{indexset, IndexMap, IndexSet};
use log::debug;

use syscare_abi::PatchStatus;
use syscare_common::{concat_os, os, util::digest};

use super::PatchOpFlag;
use crate::patch::entity::KernelPatch;

mod sys;
mod target;

use target::PatchTarget;

pub struct KernelPatchDriver {
    patch_target_map: IndexMap<OsString, PatchTarget>,
}

impl KernelPatchDriver {
    pub fn new() -> Result<Self> {
        Ok(Self {
            patch_target_map: IndexMap::new(),
        })
    }
}

impl KernelPatchDriver {
    fn check_consistency(patch: &KernelPatch) -> Result<()> {
        let patch_file = patch.patch_file.as_path();
        let real_checksum = digest::file(patch_file)?;
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
        for kmod_name in Self::parse_target_modules(patch) {
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

    fn check_conflict_symbols(&self, patch: &KernelPatch) -> Result<()> {
        let mut conflict_patches = indexset! {};

        let target_symbols = PatchTarget::classify_symbols(&patch.symbols);
        for (target_name, symbols) in target_symbols {
            if let Some(target) = self.patch_target_map.get(target_name) {
                conflict_patches.extend(
                    target
                        .get_conflicts(symbols)
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

    fn check_override_symbols(&self, patch: &KernelPatch) -> Result<()> {
        let mut override_patches = indexset! {};

        let target_symbols = PatchTarget::classify_symbols(&patch.symbols);
        for (target_name, symbols) in target_symbols {
            if let Some(target) = self.patch_target_map.get(target_name) {
                override_patches.extend(
                    target
                        .get_overrides(&patch.uuid, symbols)
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
    fn parse_target_modules(patch: &KernelPatch) -> impl IntoIterator<Item = &OsStr> {
        patch.symbols.iter().map(|symbol| symbol.target.as_os_str())
    }

    fn add_patch_symbols(&mut self, patch: &KernelPatch) {
        let target_symbols = PatchTarget::classify_symbols(&patch.symbols);

        for (target_name, symbols) in target_symbols {
            let target = self
                .patch_target_map
                .entry(target_name.to_os_string())
                .or_insert_with(|| PatchTarget::new(target_name));

            target.add_symbols(patch.uuid, symbols);
        }
    }

    fn remove_patch_symbols(&mut self, patch: &KernelPatch) {
        let target_symbols = PatchTarget::classify_symbols(&patch.symbols);

        for (target_name, symbols) in target_symbols {
            if let Some(target) = self.patch_target_map.get_mut(target_name) {
                target.remove_symbols(&patch.uuid, symbols);
            }
        }
    }
}

impl KernelPatchDriver {
    pub fn status(&self, patch: &KernelPatch) -> Result<PatchStatus> {
        sys::read_patch_status(patch)
    }

    pub fn check(&self, patch: &KernelPatch, flag: PatchOpFlag) -> Result<()> {
        if flag == PatchOpFlag::Force {
            return Ok(());
        }

        Self::check_consistency(patch)?;
        Self::check_compatiblity(patch)?;
        Self::check_dependency(patch)?;

        Ok(())
    }

    pub fn apply(&mut self, patch: &KernelPatch) -> Result<()> {
        let selinux_status = os::selinux::get_status()?;
        if selinux_status == os::selinux::Status::Enforcing {
            debug!("SELinux is enforcing");
            sys::set_security_attribute(&patch.patch_file)
                .context("Kpatch: Failed to set security attribute")?;
        }

        sys::apply_patch(patch)
    }

    pub fn remove(&mut self, patch: &KernelPatch) -> Result<()> {
        sys::remove_patch(patch)
    }

    pub fn active(&mut self, patch: &KernelPatch, flag: PatchOpFlag) -> Result<()> {
        if flag != PatchOpFlag::Force {
            self.check_conflict_symbols(patch)?;
        }

        sys::active_patch(patch)?;
        self.add_patch_symbols(patch);

        Ok(())
    }

    pub fn deactive(&mut self, patch: &KernelPatch, flag: PatchOpFlag) -> Result<()> {
        if flag != PatchOpFlag::Force {
            self.check_override_symbols(patch)?;
        }

        sys::deactive_patch(patch)?;
        self.remove_patch_symbols(patch);

        Ok(())
    }
}
