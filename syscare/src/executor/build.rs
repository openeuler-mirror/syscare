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

use anyhow::{anyhow, Context, Result};

use std::process::{exit, Command};

use super::CommandExecutor;
use crate::args::SubCommand;

const SYSCARE_BUILD_NAME: &str = "syscare-build";
const SYSCARE_BUILD_PATH: &str = "/usr/libexec/syscare/syscare-build";

pub struct BuildCommandExecutor;

impl BuildCommandExecutor {
    fn exec_patch_build_cmd(args: &[String]) -> std::io::Result<i32> {
        Ok(Command::new(SYSCARE_BUILD_PATH)
            .args(args)
            .spawn()?
            .wait()?
            .code()
            .expect("Failed to get process exit code"))
    }
}

impl CommandExecutor for BuildCommandExecutor {
    fn invoke(&self, command: &SubCommand) -> Result<()> {
        if let SubCommand::Build { args } = command {
            let exit_code = Self::exec_patch_build_cmd(args)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        anyhow!("Command \"{}\" is not installed", SYSCARE_BUILD_NAME)
                    }
                    _ => e.into(),
                })
                .with_context(|| format!("Failed to start \"{}\" process", SYSCARE_BUILD_NAME))?;

            exit(exit_code);
        }

        Ok(())
    }
}
