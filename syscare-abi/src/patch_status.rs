// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-abi is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use serde::{Deserialize, Serialize};

const PATCH_STATUS_UNKNOWN: &str = "UNKNOWN";
const PATCH_STATUS_NOT_APPLIED: &str = "NOT-APPLIED";
const PATCH_STATUS_DEACTIVED: &str = "DEACTIVED";
const PATCH_STATUS_ACTIVED: &str = "ACTIVED";
const PATCH_STATUS_ACCEPTED: &str = "ACCEPTED";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PatchStatus {
    Unknown,
    NotApplied,
    Deactived,
    Actived,
    Accepted,
}

impl Default for PatchStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for PatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PatchStatus::Unknown => PATCH_STATUS_UNKNOWN,
            PatchStatus::NotApplied => PATCH_STATUS_NOT_APPLIED,
            PatchStatus::Deactived => PATCH_STATUS_DEACTIVED,
            PatchStatus::Actived => PATCH_STATUS_ACTIVED,
            PatchStatus::Accepted => PATCH_STATUS_ACCEPTED,
        })
    }
}
