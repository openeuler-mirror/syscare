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

use super::function::{rpc, RpcResult};

#[rpc(server)]
pub trait Skeleton {
    #[rpc(name = "hook_compiler")]
    fn hook_compiler(&self, exec_path: PathBuf) -> RpcResult<()>;

    #[rpc(name = "unhook_compiler")]
    fn unhook_compiler(&self, exec_path: PathBuf) -> RpcResult<()>;
}
