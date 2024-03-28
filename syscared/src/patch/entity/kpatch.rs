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

use std::{ffi::OsString, path::PathBuf, sync::Arc};

use syscare_abi::{PatchInfo, PatchType};
use uuid::Uuid;

/// Kernel patch symbol definition
#[derive(Clone)]
pub struct KernelPatchSymbol {
    pub name: OsString,
    pub target: OsString,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
}

impl std::fmt::Debug for KernelPatchSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KernelPatchSymbol")
            .field("name", &self.name)
            .field("target", &self.target)
            .field("old_addr", &format!("{:#x}", self.old_addr))
            .field("old_size", &format!("{:#x}", self.old_size))
            .field("new_addr", &format!("{:#x}", self.new_addr))
            .field("new_size", &format!("{:#x}", self.new_size))
            .finish()
    }
}

impl std::fmt::Display for KernelPatchSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
             f,
             "name: {}, target: {}, old_addr: {:#x}, old_size: {:#x}, new_addr: {:#x}, new_size: {:#x}",
             self.name.to_string_lossy(),
             self.target.to_string_lossy(),
             self.old_addr,
             self.old_size,
             self.new_addr,
             self.new_size,
         )
    }
}

/// Kernel patch definition
#[derive(Debug)]
pub struct KernelPatch {
    pub uuid: Uuid,
    pub name: OsString,
    pub kind: PatchType,
    pub info: Arc<PatchInfo>,
    pub pkg_name: String,
    pub module_name: OsString,
    pub symbols: Vec<KernelPatchSymbol>,
    pub patch_file: PathBuf,
    pub sys_file: PathBuf,
    pub checksum: String,
}
