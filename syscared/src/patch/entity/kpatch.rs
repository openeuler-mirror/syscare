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

use syscare_abi::PatchInfo;
use uuid::Uuid;

/// Kernel patch function definition
#[derive(Clone)]
pub struct KernelPatchFunction {
    pub name: OsString,
    pub object: OsString,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
}

impl std::fmt::Debug for KernelPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KernelPatchFunction")
            .field("name", &self.name)
            .field("object", &self.object)
            .field("old_addr", &format!("{:#x}", self.old_addr))
            .field("old_size", &format!("{:#x}", self.old_size))
            .field("new_addr", &format!("{:#x}", self.new_addr))
            .field("new_size", &format!("{:#x}", self.new_size))
            .finish()
    }
}

impl std::fmt::Display for KernelPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
             f,
             "name: {}, object: {}, old_addr: {:#x}, old_size: {:#x}, new_addr: {:#x}, new_size: {:#x}",
             self.name.to_string_lossy(),
             self.object.to_string_lossy(),
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
    pub info: Arc<PatchInfo>,
    pub pkg_name: String,
    pub module_name: OsString,
    pub target_name: OsString,
    pub functions: Vec<KernelPatchFunction>,
    pub patch_file: PathBuf,
    pub sys_file: PathBuf,
    pub checksum: String,
}
