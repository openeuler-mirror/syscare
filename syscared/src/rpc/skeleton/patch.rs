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

use syscare_abi::{PackageInfo, PatchInfo, PatchListRecord, PatchStateRecord};

use super::function::{rpc, RpcResult};

#[rpc(server)]
pub trait PatchSkeleton {
    #[rpc(name = "check_patch")]
    fn check_patch(&self, identifier: String) -> RpcResult<()>;

    #[rpc(name = "apply_patch")]
    fn apply_patch(&self, identifier: String, force: bool) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "remove_patch")]
    fn remove_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "active_patch")]
    fn active_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "deactive_patch")]
    fn deactive_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "accept_patch")]
    fn accept_patch(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "get_patch_list")]
    fn get_patch_list(&self) -> RpcResult<Vec<PatchListRecord>>;

    #[rpc(name = "get_patch_status")]
    fn get_patch_status(&self, identifier: String) -> RpcResult<Vec<PatchStateRecord>>;

    #[rpc(name = "get_patch_info")]
    fn get_patch_info(&self, identifier: String) -> RpcResult<PatchInfo>;

    #[rpc(name = "get_patch_target")]
    fn get_patch_target(&self, identifier: String) -> RpcResult<PackageInfo>;

    #[rpc(name = "save_patch_status")]
    fn save_patch_status(&self) -> RpcResult<()>;

    #[rpc(name = "restore_patch_status")]
    fn restore_patch_status(&self, accepted_only: bool) -> RpcResult<()>;
}
