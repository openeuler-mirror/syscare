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

use std::path::{Path, PathBuf};

use anyhow::Result;
use syscare_common::fs;

const BUILD_DIR_NAME: &str = "BUILD";
const BUILDROOT_DIR_NAME: &str = "BUILDROOT";
const RPMS_DIR_NAME: &str = "RPMS";
const SOURCES_DIR_NAME: &str = "SOURCES";
const SPECS_DIR_NAME: &str = "SPECS";
const SRPMS_DIR_NAME: &str = "SRPMS";

#[derive(Debug, Clone)]
pub struct PackageBuildRoot {
    pub path: PathBuf,
    pub build: PathBuf,
    pub buildroot: PathBuf,
    pub rpms: PathBuf,
    pub sources: PathBuf,
    pub specs: PathBuf,
    pub srpms: PathBuf,
}

impl PackageBuildRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let build = path.join(BUILD_DIR_NAME);
        let buildroot = path.join(BUILDROOT_DIR_NAME);
        let rpms = path.join(RPMS_DIR_NAME);
        let sources = path.join(SOURCES_DIR_NAME);
        let specs = path.join(SPECS_DIR_NAME);
        let srpms = path.join(SRPMS_DIR_NAME);

        fs::create_dir_all(&path)?;
        fs::create_dir_all(&build)?;
        fs::create_dir_all(&buildroot)?;
        fs::create_dir_all(&rpms)?;
        fs::create_dir_all(&sources)?;
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&srpms)?;

        Ok(Self {
            path,
            build,
            buildroot,
            rpms,
            sources,
            specs,
            srpms,
        })
    }
}

impl AsRef<Path> for PackageBuildRoot {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
