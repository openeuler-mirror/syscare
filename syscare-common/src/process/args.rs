// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::ffi::{OsStr, OsString};

#[derive(Clone, Default)]
pub struct CommandArgs {
    args: Vec<OsString>,
}

impl CommandArgs {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn arg<S>(&mut self, arg: S) -> &mut Self
    where
        S: AsRef<OsStr>,
    {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
}

impl IntoIterator for CommandArgs {
    type Item = OsString;

    type IntoIter = std::vec::IntoIter<OsString>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter()
    }
}
