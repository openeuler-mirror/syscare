// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{path::Path, rc::Rc};

use anyhow::Result;
use function_name::named;

use super::{args::RpcArguments, remote::RpcRemote};

#[derive(Clone)]
pub struct UpatchProxy {
    remote: Rc<RpcRemote>,
}

impl UpatchProxy {
    pub fn new(remote: Rc<RpcRemote>) -> Self {
        Self { remote }
    }

    #[named]
    pub fn hook_compiler<P: AsRef<Path>>(&self, exec_path: P) -> Result<()> {
        self.remote.call_with_args(
            function_name!(),
            RpcArguments::new().arg(exec_path.as_ref().to_path_buf()),
        )
    }

    #[named]
    pub fn unhook_compiler<P: AsRef<Path>>(&self, exec_path: P) -> Result<()> {
        self.remote.call_with_args(
            function_name!(),
            RpcArguments::new().arg(exec_path.as_ref().to_path_buf()),
        )
    }
}
