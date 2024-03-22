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

use std::ffi::OsStr;

use syscare_abi::PatchInfo;
use uuid::Uuid;

use super::{KernelPatch, UserPatch};

/// Patch definition
#[derive(Debug)]
pub enum Patch {
    KernelPatch(KernelPatch),
    UserPatch(UserPatch),
}

impl Patch {
    pub fn uuid(&self) -> &Uuid {
        match self {
            Patch::KernelPatch(patch) => &patch.uuid,
            Patch::UserPatch(patch) => &patch.uuid,
        }
    }

    pub fn name(&self) -> &OsStr {
        match self {
            Patch::KernelPatch(patch) => patch.name.as_os_str(),
            Patch::UserPatch(patch) => patch.name.as_os_str(),
        }
    }

    pub fn pkg_name(&self) -> &str {
        match self {
            Patch::KernelPatch(patch) => patch.pkg_name.as_str(),
            Patch::UserPatch(patch) => patch.pkg_name.as_str(),
        }
    }

    pub fn info(&self) -> &PatchInfo {
        match self {
            Patch::KernelPatch(patch) => patch.info.as_ref(),
            Patch::UserPatch(patch) => patch.info.as_ref(),
        }
    }
}

impl std::cmp::PartialEq for Patch {
    fn eq(&self, other: &Self) -> bool {
        self.uuid() == other.uuid()
    }
}

impl std::cmp::Eq for Patch {}

impl std::cmp::PartialOrd for Patch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Patch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name().cmp(other.name())
    }
}

impl std::fmt::Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name().to_string_lossy())
    }
}
