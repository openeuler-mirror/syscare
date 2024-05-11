// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
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

use anyhow::Result;
use clap::{AppSettings, ColorChoice, Parser};
use log::LevelFilter;

use syscare_common::fs;

use super::{DAEMON_ABOUT, DAEMON_NAME, DAEMON_VERSION};

const DEFAULT_CONFIG_DIR: &str = "/etc/syscare";
const DEFAULT_WORK_DIR: &str = "/var/run/syscare";
const DEFAULT_LOG_DIR: &str = "/var/log/syscare";
const DEFAULT_LOG_LEVEL: &str = "info";

#[derive(Debug, Clone, Parser)]
#[clap(
    bin_name = DAEMON_NAME,
    version = DAEMON_VERSION,
    about = DAEMON_ABOUT,
    color(ColorChoice::Never),
    global_setting(AppSettings::DeriveDisplayOrder),
    term_width(120),
)]

pub struct Arguments {
    /// Run as a daemon
    #[clap(short, long)]
    pub daemon: bool,

    /// Daemon config directory
    #[clap(long, default_value=DEFAULT_CONFIG_DIR)]
    pub config_dir: PathBuf,

    /// Daemon working directory
    #[clap(long, default_value=DEFAULT_WORK_DIR)]
    pub work_dir: PathBuf,

    #[clap(long, default_value=DEFAULT_LOG_DIR)]
    /// Daemon logging directory
    pub log_dir: PathBuf,

    /// Set the logging level ("trace"|"debug"|"info"|"warn"|"error")
    #[clap(short, long, default_value=DEFAULT_LOG_LEVEL)]
    pub log_level: LevelFilter,
}

impl Arguments {
    pub fn new() -> Result<Self> {
        Self::parse().normalize_path()
    }

    fn normalize_path(mut self) -> Result<Self> {
        self.config_dir = fs::normalize(&self.config_dir)?;
        self.work_dir = fs::normalize(self.work_dir)?;
        self.log_dir = fs::normalize(&self.log_dir)?;

        Ok(self)
    }
}

impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
