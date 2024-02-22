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

use anyhow::{Context, Result};

use crate::fast_reboot::{KExecManager, RebootOption};
use log::info;

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::FastRebootSkeleton,
};

pub struct FastRebootSkeletonImpl;

impl FastRebootSkeleton for FastRebootSkeletonImpl {
    fn fast_reboot(&self, kernel_version: Option<String>, force: bool) -> RpcResult<()> {
        RpcFunction::call(move || -> Result<()> {
            info!("Rebooting system...");

            KExecManager::load_kernel(kernel_version)
                .and_then(|_| {
                    KExecManager::execute(match force {
                        true => RebootOption::Forced,
                        false => RebootOption::Normal,
                    })
                })
                .context("Failed to reboot system")
        })
    }
}
