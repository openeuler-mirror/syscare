// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * metadata-viewer is licensed under Mulan PSL v2.
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

use anyhow::{ensure, Result};
use clap::{AppSettings, ColorChoice, Parser};

use syscare_common::fs;

use crate::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

#[derive(Debug, Parser)]
#[clap(
    bin_name = CLI_NAME,
    version = CLI_VERSION,
    about = CLI_ABOUT,
    arg_required_else_help(true),
    color(ColorChoice::Never),
    global_setting(AppSettings::DeriveDisplayOrder),
    term_width(120),
)]
pub struct Arguments {
    /// Metdata file
    pub files: Vec<PathBuf>,
    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize()?.check()
    }

    fn normalize(mut self) -> Result<Self> {
        for file in &mut self.files {
            *file = fs::normalize(&file)?;
        }
        Ok(self)
    }

    fn check(self) -> Result<Self> {
        for file in &self.files {
            ensure!(file.is_file(), "Cannot find {}", file.display())
        }
        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}
