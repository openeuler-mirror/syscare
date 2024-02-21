// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::rc::Rc;

use anyhow::Result;
use function_name::named;

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use super::{args::RpcArguments, remote::RpcRemote};

pub struct RpcProxy {
    remote: Rc<RpcRemote>,
}

impl RpcProxy {
    pub fn new(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }

    #[named]
    pub fn check_patch(&self, identifier: &str) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn apply_patch(&self, identifier: &str, force: bool) -> Result<Vec<PatchStateRecord>> {
        self.remote.call_with_args(
            function_name!(),
            RpcArguments::new().arg(identifier).arg(force),
        )
    }

    #[named]
    pub fn remove_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn active_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn deactive_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn accept_patch(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_list(&self) -> Result<Vec<PatchListRecord>> {
        self.remote.call(function_name!())
    }

    #[named]
    pub fn get_patch_status(&self, identifier: &str) -> Result<Vec<PatchStateRecord>> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_info(&self, identifier: &str) -> Result<PatchInfo> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn get_patch_target(&self, identifier: &str) -> Result<PackageInfo> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(identifier))
    }

    #[named]
    pub fn save_patch_status(&self) -> Result<()> {
        self.remote.call(function_name!())
    }

    #[named]
    pub fn restore_patch_status(&self, accepted_only: bool) -> Result<()> {
        self.remote
            .call_with_args(function_name!(), RpcArguments::new().arg(accepted_only))
    }
}
