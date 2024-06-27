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

mod package_root;
mod patch_root;

pub use package_root::*;
pub use patch_root::*;

const PACKAGE_ROOT_NAME: &str = "package";
const PATCH_ROOT_NAME: &str = "patch";
const BUILD_LOG_NAME: &str = "build.log";

#[derive(Debug, Clone)]
pub struct BuildRoot {
    pub path: PathBuf,
    pub package: PackageRoot,
    pub patch: PatchRoot,
    pub log_file: PathBuf,
}

impl BuildRoot {
    pub fn new<P: AsRef<Path>>(directory: P) -> Result<Self> {
        let path = directory.as_ref().to_path_buf();
        let package = PackageRoot::new(path.join(PACKAGE_ROOT_NAME))?;
        let patch = PatchRoot::new(path.join(PATCH_ROOT_NAME))?;
        let log_file = path.join(BUILD_LOG_NAME);
        fs::create_dir_all(&path)?;

        Ok(Self {
            path,
            log_file,
            patch,
            package,
        })
    }

    pub fn remove(&self) -> Result<()> {
        fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}
