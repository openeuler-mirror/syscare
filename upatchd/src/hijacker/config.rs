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

use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use syscare_common::util::fs;

const CC_BINARY: &str = "/usr/bin/cc";
const CXX_BINARY: &str = "/usr/bin/c++";
const GCC_BINARY: &str = "/usr/bin/gcc";
const GXX_BINARY: &str = "/usr/bin/g++";
const AS_BINARY: &str = "/usr/bin/as";

const CC_HIJACKER: &str = "/usr/libexec/syscare/cc-hijacker";
const CXX_HIJACKER: &str = "/usr/libexec/syscare/c++-hijacker";
const GCC_HIJACKER: &str = "/usr/libexec/syscare/gcc-hijacker";
const GXX_HIJACKER: &str = "/usr/libexec/syscare/g++-hijacker";
const AS_HIJACKER: &str = "/usr/libexec/syscare/as-hijacker";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HijackerConfig(HashMap<PathBuf, PathBuf>);

impl HijackerConfig {
    pub fn parse_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref();
        let config_file = fs::open_file(config_path)?;
        let instance: Self = serde_yaml::from_reader(config_file)
            .map_err(|_| anyhow!("Failed to parse config \"{}\"", config_path.display()))?;

        Ok(instance)
    }

    pub fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config_path = path.as_ref();
        let config_file = fs::create_file(config_path)?;
        serde_yaml::to_writer(config_file, self)
            .map_err(|_| anyhow!("Failed to write config \"{}\"", config_path.display()))?;

        Ok(())
    }
}

impl Deref for HijackerConfig {
    type Target = HashMap<PathBuf, PathBuf>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for HijackerConfig {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert(PathBuf::from(CC_BINARY), PathBuf::from(CC_HIJACKER));
        map.insert(PathBuf::from(CXX_BINARY), PathBuf::from(CXX_HIJACKER));
        map.insert(PathBuf::from(GCC_BINARY), PathBuf::from(GCC_HIJACKER));
        map.insert(PathBuf::from(GXX_BINARY), PathBuf::from(GXX_HIJACKER));
        map.insert(PathBuf::from(AS_BINARY), PathBuf::from(AS_HIJACKER));

        Self(map)
    }
}

impl std::fmt::Display for HijackerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", &self.0))
    }
}

#[test]
fn test() -> Result<()> {
    use anyhow::{ensure, Context};

    let tmp_file = PathBuf::from("/tmp/upatch_hijacker_config.yaml");

    let orig_cfg = HijackerConfig::default();
    println!("{}", orig_cfg);

    orig_cfg
        .write_to(&tmp_file)
        .context("Failed to write config")?;

    let new_cfg = HijackerConfig::parse_from(tmp_file).context("Failed to read config")?;
    println!("{}", new_cfg);

    ensure!(orig_cfg == new_cfg, "Config does not match");

    Ok(())
}
