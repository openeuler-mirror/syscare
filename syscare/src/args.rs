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

use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::{AppSettings, ColorChoice, Parser, Subcommand};

use syscare_common::fs;

use super::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_WORK_DIR: &str = "/var/run/syscare";

#[derive(Parser, Debug)]
#[clap(
    bin_name = CLI_NAME,
    version = CLI_VERSION,
    about = CLI_ABOUT,
    allow_external_subcommands(true),
    arg_required_else_help(true),
    color(ColorChoice::Never),
    disable_help_subcommand(true),
    global_setting(AppSettings::DeriveDisplayOrder),
    term_width(120),
)]
pub struct Arguments {
    /// Command name
    #[clap(subcommand)]
    pub subcommand: SubCommand,

    /// Path for working directory
    #[clap(short, long, default_value=DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    /// Provide more detailed info
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SubCommand {
    /// Show patch info
    Info {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Show patch target
    Target {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Show patch status
    Status {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// List all patches
    List,
    /// Check a patch
    Check {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Apply a patch
    Apply {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
        /// Force to apply a patch
        #[clap(short, long)]
        force: bool,
    },
    /// Remove a patch
    Remove {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Active a patch
    Active {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
        /// Force to active a patch
        #[clap(short, long)]
        force: bool,
    },
    /// Deactive a patch
    Deactive {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Accept a patch
    Accept {
        /// Patch identifier
        #[clap(required = true)]
        identifiers: Vec<String>,
    },
    /// Save all patch status
    Save,
    /// Restore all patch status
    Restore {
        /// Only restore ACCEPTED patches
        #[clap(long)]
        accepted: bool,
    },
    /// Rescan all patches
    Rescan,
    /// External subcommand
    #[clap(external_subcommand)]
    External(Vec<OsString>),
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize_path()
    }

    fn normalize_path(mut self) -> Result<Self> {
        self.work_dir = fs::normalize(&self.work_dir)?;

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
