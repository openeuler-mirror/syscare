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

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::{AppSettings, ColorChoice, Parser};
use syscare_common::fs;

use super::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_BUILD_ROOT: &str = ".";
const DEFAULT_BUILD_PATCH_CMD: &str = "";
const DEFAULT_COMPILERS: &str = "cc";
const DEFAULT_OUTPUT_DIR: &str = ".";

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
    /// Specify output name
    #[clap(short, long, default_value = "", hide_default_value = true)]
    pub name: OsString,

    /// Specify working directory
    #[clap(long, default_value = DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    /// Specify build temporary directory
    #[clap(long, default_value = DEFAULT_BUILD_ROOT)]
    pub build_root: PathBuf,

    /// Specify source directory
    #[clap(short, long)]
    pub source_dir: PathBuf,

    /// Specify build source command
    #[clap(short, long)]
    pub build_source_cmd: String,

    /// Specify build patched source command [default: <BUILD_SOURCE_CMD>]
    #[clap(long, default_value = DEFAULT_BUILD_PATCH_CMD, hide_default_value = true)]
    pub build_patch_cmd: String,

    /// Specify debuginfo files
    #[clap(short, long, multiple = true, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Specify the directory of searching elf [default: <SOURCE_DIR>]
    #[clap(long, default_value = "", hide_default_value = true)]
    pub elf_dir: PathBuf,

    /// Specify elf's relative path relate to 'elf' or absolute patch list
    #[clap(long, multiple = true, required = true)]
    pub elf: Vec<PathBuf>,

    /// Specify compiler(s)
    #[clap(short, long, multiple = true, default_value = DEFAULT_COMPILERS)]
    pub compiler: Vec<PathBuf>,

    /// Patch file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub patch: Vec<PathBuf>,

    /// Specify output directory [default: <WORK_DIR>]
    #[clap(short, long, default_value = DEFAULT_OUTPUT_DIR, hide_default_value = true)]
    pub output_dir: PathBuf,

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
        let mut args = Self::parse().normalize_path()?.check()?;

        if !args.name.is_empty() {
            args.name.push("-");
        }

        // args.build_root = args.build_root.join("upatch");
        if args.build_patch_cmd.is_empty() {
            args.build_patch_cmd = args.build_source_cmd.clone();
        }

        args.elf_dir = match args.elf_dir.as_os_str().is_empty() {
            false => fs::normalize(&args.elf_dir)?,
            true => args.source_dir.clone(),
        };

        for elf_path in &mut args.elf {
            *elf_path = args.elf_dir.join(&elf_path);
        }

        Ok(args)
    }

    fn normalize_path(mut self) -> Result<Self> {
        self.work_dir = fs::normalize(&self.work_dir)?;
        self.build_root = fs::normalize(self.build_root)?;
        self.source_dir = fs::normalize(&self.source_dir)?;
        self.elf_dir = fs::normalize(&self.elf_dir)?;
        self.output_dir = fs::normalize(&self.output_dir)?;

        for debuginfo in &mut self.debuginfo {
            *debuginfo = fs::normalize(&debuginfo)?;
        }
        for patch in &mut self.patch {
            *patch = fs::normalize(&patch)?;
        }

        Ok(self)
    }

    fn check(self) -> Result<Self> {
        ensure!(
            self.work_dir.is_dir(),
            format!("Cannot find working directory {}", self.work_dir.display())
        );
        ensure!(
            self.source_dir.is_dir(),
            format!("Cannot find source directory {}", self.source_dir.display())
        );
        ensure!(
            self.elf_dir.is_dir(),
            format!("Cannot find elf directory {}", self.elf_dir.display())
        );
        for debuginfo in &self.debuginfo {
            ensure!(
                debuginfo.is_file(),
                format!("Cannot find debuginfo {}", debuginfo.display())
            );
        }
        for patch in &self.patch {
            ensure!(
                patch.is_file(),
                format!("Cannot find patch {}", patch.display())
            );
        }
        ensure!(
            self.elf.len() == self.debuginfo.len(),
            "Cannot match the debuginfo corresponds to elf files"
        );

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
