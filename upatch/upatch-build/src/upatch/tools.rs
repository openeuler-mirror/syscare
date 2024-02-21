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

use std::path::{Path, PathBuf};

use crate::cmd::*;
use crate::tool::*;

use super::Error;
use super::Result;

const SUPPORT_DIFF: &str = "upatch-diff";
pub struct Tool {
    diff: PathBuf,
}

impl Tool {
    pub fn new() -> Self {
        Self {
            diff: PathBuf::new(),
        }
    }

    pub fn check(&mut self) -> std::io::Result<()> {
        self.diff = search_tool(SUPPORT_DIFF)?;
        Ok(())
    }

    pub fn upatch_diff<P, Q, O, D, L>(
        &self,
        source: P,
        patch: Q,
        output: O,
        debug_info: D,
        log_file: L,
        verbose: bool,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        O: AsRef<Path>,
        D: AsRef<Path>,
        L: AsRef<Path>,
    {
        let mut args_list = ExternCommandArgs::new()
            .arg("-s")
            .arg(source.as_ref())
            .arg("-p")
            .arg(patch.as_ref())
            .arg("-o")
            .arg(output.as_ref())
            .arg("-r")
            .arg(debug_info.as_ref());
        if verbose {
            args_list = args_list.arg("-d");
        }
        let output = ExternCommand::new(&self.diff).execv(args_list)?;
        if !output.exit_status().success() {
            return Err(Error::Diff(format!(
                "{}: please look {:?} for detail.",
                output.exit_code(),
                log_file.as_ref()
            )));
        };
        Ok(())
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self::new()
    }
}
