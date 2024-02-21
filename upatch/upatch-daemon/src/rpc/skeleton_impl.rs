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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{debug, info};

use crate::hijacker::Hijacker;

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::Skeleton,
};

pub struct SkeletonImpl {
    hijacker: Hijacker,
}

impl SkeletonImpl {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        debug!("Initializing hijacker...");
        Ok(Self {
            hijacker: Hijacker::new(config_path).context("Failed to initialize hijacker")?,
        })
    }
}

impl Skeleton for SkeletonImpl {
    fn enable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Enable hijack: \"{}\"", elf_path.display());
            self.hijacker
                .hijack(&elf_path)
                .with_context(|| format!("Failed to hijack \"{}\"", elf_path.display()))
        })
    }

    fn disable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Disable hijack: \"{}\"", elf_path.display());
            self.hijacker
                .release(&elf_path)
                .with_context(|| format!("Failed to release hijack for \"{}\"", elf_path.display()))
        })
    }
}
