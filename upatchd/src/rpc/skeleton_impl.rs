// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{debug, info};

use crate::helper::{UpatchHelper, UpatchHelperConfig};

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::Skeleton,
};

pub struct SkeletonImpl {
    helper: UpatchHelper,
}

impl SkeletonImpl {
    pub fn new(config: UpatchHelperConfig) -> Result<Self> {
        debug!("Initializing upatch helper...");
        Ok(Self {
            helper: UpatchHelper::new(config).context("Failed to initialize upatch helper")?,
        })
    }
}

impl Skeleton for SkeletonImpl {
    fn hook_compiler(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Hook compiler: {}", elf_path.display());
            self.helper
                .register_hooker(&elf_path)
                .with_context(|| format!("Failed to hook helper {}", elf_path.display()))
        })
    }

    fn unhook_compiler(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Unhook compiler: {}", elf_path.display());
            self.helper
                .unregister_hooker(&elf_path)
                .with_context(|| format!("Failed to unhook compiler {}", elf_path.display()))
        })
    }
}
