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

/// User patch function definition
#[derive(Clone)]
pub struct UserPatchFunction {
    pub name: OsString,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
}

impl std::fmt::Debug for UserPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserPatchFunction")
            .field("name", &self.name)
            .field("old_addr", &format!("0x{}", self.old_addr))
            .field("old_size", &format!("0x{}", self.old_size))
            .field("new_addr", &format!("0x{}", self.new_addr))
            .field("new_size", &format!("0x{}", self.new_size))
            .finish()
    }
}

impl std::fmt::Display for UserPatchFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, old_addr: 0x{:x}, old_size: 0x{:x}, new_addr: 0x{:x}, new_size: 0x{:x}",
            self.name.to_string_lossy(),
            self.old_addr,
            self.old_size,
            self.new_addr,
            self.new_size,
        )
    }
}

/// User patch definition
#[derive(Debug)]
pub struct UserPatch {
    pub uuid: Uuid,
    pub name: OsString,
    pub info: Arc<PatchInfo>,
    pub pkg_name: String,
    pub functions: Vec<UserPatchFunction>,
    pub patch_file: PathBuf,
    pub target_elf: PathBuf,
    pub checksum: String,
}
