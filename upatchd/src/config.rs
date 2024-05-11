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

use std::path::Path;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use syscare_common::fs;

use crate::hijacker::HijackerConfig;

const DEFAULT_SOCKET_UID: u32 = 0;
const DEFAULT_SOCKET_GID: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketConfig {
    pub uid: u32,
    pub gid: u32,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            uid: DEFAULT_SOCKET_UID,
            gid: DEFAULT_SOCKET_GID,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket: SocketConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub daemon: DaemonConfig,
    pub hijacker: HijackerConfig,
}

impl Config {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref();
        let instance = serde_yaml::from_reader(fs::open_file(config_path)?)
            .map_err(|_| anyhow!("Failed to parse config {}", config_path.display()))?;

        Ok(instance)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config_path = path.as_ref();
        let config_file = fs::create_file(config_path)?;
        serde_yaml::to_writer(config_file, self)
            .map_err(|_| anyhow!("Failed to write config {}", config_path.display()))?;

        Ok(())
    }
}

#[test]
fn test() -> Result<()> {
    use anyhow::{ensure, Context};
    use std::path::PathBuf;

    let tmp_file = PathBuf::from("/tmp/upatchd.yaml");

    let orig_cfg = Config::default();
    println!("{:#?}", orig_cfg);

    orig_cfg
        .write(&tmp_file)
        .context("Failed to write config")?;

    let new_cfg = Config::parse(tmp_file).context("Failed to read config")?;
    println!("{:#?}", new_cfg);

    ensure!(orig_cfg == new_cfg, "Config does not match");

    Ok(())
}
