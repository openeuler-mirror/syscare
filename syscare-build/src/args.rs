// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
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

use anyhow::{bail, ensure, Result};
use clap::{AppSettings, ColorChoice, Parser};
use lazy_static::lazy_static;

use syscare_common::{os, util::fs};

use super::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_PATCH_VERSION: &str = "1";
const DEFAULT_PATCH_RELEASE: &str = "1";
const DEFAULT_PATCH_DESCRIPTION: &str = "(none)";
const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_BUILD_ROOT: &str = ".";
const DEFAULT_OUTPUT_DIR: &str = ".";

lazy_static! {
    static ref DEFAULT_BUILD_JOBS: String = os::cpu::num().to_string();
    static ref DEFAULT_PATCH_ARCH: String = os::cpu::arch().to_string_lossy().to_string();
}

#[derive(Parser, Debug)]
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
    /// Patch name
    #[clap(short = 'n', long)]
    pub patch_name: String,

    /// Patch architecture
    #[clap(long, default_value = DEFAULT_PATCH_ARCH.as_str())]
    pub patch_arch: String,

    /// Patch version
    #[clap(long, default_value = DEFAULT_PATCH_VERSION)]
    pub patch_version: String,

    /// Patch release
    #[clap(long, default_value = DEFAULT_PATCH_RELEASE)]
    pub patch_release: u32,

    /// Patch description
    #[clap(long, default_value = DEFAULT_PATCH_DESCRIPTION)]
    pub patch_description: String,

    /// Patch requirements
    #[clap(long, multiple = true)]
    pub patch_requires: Vec<String>,

    /// Source package(s)
    #[clap(short, long, multiple = true, required = true)]
    pub source: Vec<PathBuf>,

    /// Debuginfo package(s)
    #[clap(short, long, multiple = true, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Patch file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub patch: Vec<PathBuf>,

    /// Working directory
    #[clap(long, default_value = DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    /// Build temporary directory
    #[clap(long, default_value = DEFAULT_BUILD_ROOT)]
    pub build_root: PathBuf,

    /// Output directory
    #[clap(short, long, default_value = DEFAULT_OUTPUT_DIR)]
    pub output: PathBuf,

    /// Parallel build jobs
    #[clap(short, long, default_value = DEFAULT_BUILD_JOBS.as_str())]
    pub jobs: usize,

    /// Skip compiler version check (not recommended)
    #[clap(long)]
    pub skip_compiler_check: bool,

    /// Skip post-build cleanup
    #[clap(long)]
    pub skip_cleanup: bool,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize_path().and_then(Self::check)
    }

    fn normalize_path(mut self) -> Result<Self> {
        for source_file in &mut self.source {
            *source_file = fs::normalize(&source_file)?;
        }
        for debuginfo_file in &mut self.debuginfo {
            *debuginfo_file = fs::normalize(&debuginfo_file)?;
        }
        for patch_file in &mut self.patch {
            *patch_file = fs::normalize(&patch_file)?;
        }
        self.work_dir = fs::normalize(&self.work_dir)?;
        self.build_root = fs::normalize(&self.build_root)?;
        self.output = fs::normalize(&self.output)?;

        Ok(self)
    }

    fn check(self) -> Result<Self> {
        for source_file in &self.source {
            ensure!(
                source_file.is_file(),
                format!("Cannot find file \"{}\"", source_file.display())
            );
        }
        for debuginfo_file in &self.debuginfo {
            ensure!(
                debuginfo_file.is_file(),
                format!("Cannot find file \"{}\"", debuginfo_file.display())
            );
        }
        for patch_file in &self.patch {
            ensure!(
                patch_file.is_file(),
                format!("Cannot find file \"{}\"", patch_file.display())
            );
        }
        if self.patch_arch.as_str() != os::cpu::arch() {
            bail!("Cross compilation is unsupported");
        }

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
