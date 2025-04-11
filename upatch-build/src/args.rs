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
use syscare_common::{fs, os};
use which::which;

use super::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_EMPTY_VALUE: &str = "";
const DEFAULT_SOURCE_EXT: [&str; 8] = ["h", "hpp", "hxx", "c", "cpp", "cxx", "in", "inc"];
const DEFAULT_COMPILERS: [&str; 2] = ["gcc", "g++"];
const DEFAULT_BUILD_ROOT: &str = ".";
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
    /// Patch name prefix
    #[clap(long, default_value = DEFAULT_EMPTY_VALUE)]
    pub prefix: OsString,

    /// Build temporary directory
    #[clap(long, default_value = DEFAULT_BUILD_ROOT)]
    pub build_root: PathBuf,

    /// Source directory
    #[clap(short, long)]
    pub source_dir: PathBuf,

    /// Source file extension(s)
    #[clap(long, multiple = true, default_values = &DEFAULT_SOURCE_EXT)]
    pub source_ext: Vec<OsString>,

    /// Build compiler(s)
    #[clap(long, multiple = true, default_values = &DEFAULT_COMPILERS)]
    pub compiler: Vec<PathBuf>,

    /// Build prepare command
    #[clap(long, default_value = DEFAULT_EMPTY_VALUE)]
    pub prepare_cmd: OsString,

    /// Build command
    #[clap(short('c'), long)]
    pub build_cmd: OsString,

    /// Build clean command [default: <PREPARE_CMD>]
    #[clap(long, default_value = DEFAULT_EMPTY_VALUE, hide_default_value = true)]
    pub clean_cmd: OsString,

    /// Object searching directoy [default: <SOURCE_DIR>]
    #[clap(long, default_value = DEFAULT_EMPTY_VALUE, hide_default_value = true)]
    pub object_dir: PathBuf,

    /// Binary searching directoy [default: <SOURCE_DIR>]
    #[clap(long, default_value = DEFAULT_EMPTY_VALUE, hide_default_value = true)]
    pub binary_dir: PathBuf,

    /// Binary file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub binary: Vec<OsString>,

    /// Debuginfo file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub debuginfo: Vec<PathBuf>,

    /// Patch file(s)
    #[clap(short, long, multiple = true, required = true)]
    pub patch: Vec<PathBuf>,

    /// Output directory
    #[clap(short, long, default_value = DEFAULT_OUTPUT_DIR)]
    pub output_dir: PathBuf,

    /// Keep line macro unchanged
    #[clap(long)]
    pub keep_line_macros: bool,

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
        let mut args = Self::parse();

        args.apply_defaults()
            .setup_build_root()
            .normalize()?
            .check()?;

        Ok(args)
    }

    fn setup_build_root(&mut self) -> &mut Self {
        self.build_root = self
            .build_root
            .join(format!("upatch-build.{}", os::process::id()));

        self
    }

    fn apply_defaults(&mut self) -> &mut Self {
        if self.object_dir.as_os_str().is_empty() {
            self.object_dir = self.source_dir.clone();
        }
        if self.binary_dir.as_os_str().is_empty() {
            self.binary_dir = self.source_dir.clone();
        }
        if self.clean_cmd.is_empty() {
            self.clean_cmd = self.prepare_cmd.clone();
        }

        self
    }

    fn normalize(&mut self) -> Result<&mut Self> {
        self.build_root = fs::normalize(&self.build_root)?;
        self.source_dir = fs::normalize(&self.source_dir)?;
        self.object_dir = fs::normalize(&self.object_dir)?;
        self.binary_dir = fs::normalize(&self.binary_dir)?;
        self.output_dir = fs::normalize(&self.output_dir)?;

        for compiler in &mut self.compiler {
            *compiler = which(&compiler)?;
        }
        for debuginfo in &mut self.debuginfo {
            *debuginfo = fs::normalize(&debuginfo)?;
        }
        for patch in &mut self.patch {
            *patch = fs::normalize(&patch)?;
        }

        Ok(self)
    }

    fn check(&self) -> Result<()> {
        ensure!(
            self.source_dir.is_dir(),
            format!("Cannot find source directory {}", self.source_dir.display())
        );
        for compiler in &self.compiler {
            ensure!(
                compiler.is_file(),
                format!("Cannot find compiler {}", compiler.display())
            );
        }
        for debuginfo in &self.debuginfo {
            ensure!(
                debuginfo.is_file(),
                format!("Cannot find debuginfo {}", debuginfo.display())
            );
        }
        ensure!(
            self.binary.len() == self.debuginfo.len(),
            "Cannot match the debuginfo corresponds to binary files"
        );
        for patch in &self.patch {
            ensure!(
                patch.is_file(),
                format!("Cannot find patch {}", patch.display())
            );
        }

        Ok(())
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
