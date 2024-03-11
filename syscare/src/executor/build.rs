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

use std::{os::unix::process::CommandExt, process::Command};

use anyhow::{bail, Context, Result};

use super::CommandExecutor;
use crate::args::SubCommand;

const SYSCARE_BUILD_PATH: &str = "/usr/libexec/syscare/syscare-build";

pub struct BuildCommandExecutor;

impl CommandExecutor for BuildCommandExecutor {
    fn invoke(&self, command: &SubCommand) -> Result<Option<i32>> {
        if let SubCommand::Build { args } = command {
            let e = Command::new(SYSCARE_BUILD_PATH).args(args).exec();

            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    bail!("Package syscare-build is not installed");
                }
                _ => {
                    return Err(e)
                        .with_context(|| format!("Failed to start {}", SYSCARE_BUILD_PATH))
                }
            }
        }

        Ok(None)
    }
}
