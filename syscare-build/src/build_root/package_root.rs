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

use crate::package::PackageBuildRoot;

const SOURCE_DIR_NAME: &str = "source";
const DEBUGINFO_DIR_NAME: &str = "debuginfo";
const BUILD_ROOT_DIR_NAME: &str = "patch";

#[derive(Debug, Clone)]
pub struct PackageRoot {
    pub path: PathBuf,
    pub source: PathBuf,
    pub debuginfo: PathBuf,
    pub build_root: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let source = path.join(SOURCE_DIR_NAME);
        let debuginfo = path.join(DEBUGINFO_DIR_NAME);
        let build_root = PackageBuildRoot::new(path.join(BUILD_ROOT_DIR_NAME))?;

        fs::create_dir_all(&path)?;
        fs::create_dir_all(&source)?;
        fs::create_dir_all(&debuginfo)?;

        Ok(Self {
            path,
            source,
            debuginfo,
            build_root,
        })
    }
}
