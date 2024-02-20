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

use jsonrpc::serde_json::value::RawValue;
use serde::Serialize;

use std::ops::Deref;

#[derive(Debug, Default)]
pub struct RpcArguments {
    args: Vec<Box<RawValue>>,
}

impl RpcArguments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg<T: Serialize>(mut self, arg: T) -> Self {
        self.args.push(jsonrpc::arg(arg));
        self
    }
}

impl Deref for RpcArguments {
    type Target = [Box<RawValue>];

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}
