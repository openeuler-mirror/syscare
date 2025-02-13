// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * metadata-generator is licensed under Mulan PSL v2.
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
use lazy_static::lazy_static;

use syscare_abi::Uuid;
use syscare_common::{fs, os};

use crate::{CLI_ABOUT, CLI_NAME, CLI_VERSION};

const DEFAULT_PATCH_VERSION: &str = "1";
const DEFAULT_PATCH_RELEASE: &str = "1";
const DEFAULT_ARG_NONE: &str = "(none)";
const DEFAULT_OUTPUT_DIR: &str = ".";

lazy_static! {
    static ref DEFAULT_PATCH_ARCH: String = os::cpu::arch().to_string_lossy().to_string();
}

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
    /// Patch name
    #[clap(short('n'), long)]
    pub name: String,

    /// Patch uuid
    #[clap(long, multiple = true)]
    pub uuid: Vec<Uuid>,

    /// Patch version
    #[clap(long, default_value = DEFAULT_PATCH_VERSION)]
    pub version: String,

    /// Patch release
    #[clap(long, default_value = DEFAULT_PATCH_RELEASE)]
    pub release: u32,

    /// Patch architecture
    #[clap(long, default_value = &DEFAULT_PATCH_ARCH)]
    pub arch: String,

    /// Patch target package
    #[clap(short('s'), long)]
    pub target: String,

    /// Patch entity target(s)
    #[clap(short('t'), long, multiple = true, required = true)]
    pub entity_target: Vec<PathBuf>,

    /// Patch entity patch(es)
    #[clap(short('p'), long, multiple = true, required = true)]
    pub entity_patch: Vec<PathBuf>,

    /// Patch license
    #[clap(long, default_value = DEFAULT_ARG_NONE)]
    pub license: String,

    /// Patch description
    #[clap(long, default_value = DEFAULT_ARG_NONE)]
    pub description: String,

    /// Patch file(s)
    #[clap(short('f'), long, multiple = true, required = true)]
    pub patch_file: Vec<PathBuf>,

    /// Output directory
    #[clap(short('o'), long, default_value = DEFAULT_OUTPUT_DIR)]
    pub output_dir: PathBuf,

    /// Provide more detailed info
    #[clap(short('v'), long)]
    pub verbose: bool,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize()?.generate_uuids().check()
    }

    fn normalize(mut self) -> Result<Self> {
        for entity_target in &mut self.entity_target {
            *entity_target = fs::normalize(&entity_target)?;
        }
        for entity_patch in &mut self.entity_patch {
            *entity_patch = fs::normalize(&entity_patch)?;
        }
        for patch_file in &mut self.patch_file {
            *patch_file = fs::normalize(&patch_file)?;
        }
        self.output_dir = fs::normalize(&self.output_dir)?;

        Ok(self)
    }

    fn generate_uuids(mut self) -> Self {
        if self.uuid.is_empty() {
            // Uuids would contains patch uuid & entity uuids
            let uuid_num = self.entity_target.len() + 1;
            let new_uuids = (0..uuid_num).map(|_| Uuid::new_v4()).collect();

            self.uuid = new_uuids;
        }
        self
    }

    fn check(self) -> Result<Self> {
        ensure!(
            self.uuid.len() == (self.entity_target.len() + 1),
            "Argument length of '--uuid' and '--entity-target' does not match"
        );
        ensure!(
            self.entity_target.len() == self.entity_patch.len(),
            "Argument length of '--entity-target' and '--entity-patch' does not match"
        );

        /* Would not check entity target, since it may be not a valid path */
        for entity_patch in &self.entity_patch {
            ensure!(
                entity_patch.is_file(),
                "Cannot find {}",
                entity_patch.display()
            )
        }
        for patch_file in &self.patch_file {
            ensure!(patch_file.is_file(), "Cannot find {}", patch_file.display())
        }

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}
