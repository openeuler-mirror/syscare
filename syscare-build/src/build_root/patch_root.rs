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

use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use super::util;

const BUILD_DIR_NAME: &str = "build";
const OUTPUT_DIR_NAME: &str = "output";

#[derive(Debug, Clone)]
pub struct PatchRoot {
    pub path: PathBuf,
    pub build: PathBuf,
    pub output: PathBuf,
}

impl PatchRoot {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let path = base_dir.as_ref().to_path_buf();
        let build = path.join(BUILD_DIR_NAME);
        let output = path.join(OUTPUT_DIR_NAME);

        util::create_dir_all(&path)?;
        util::create_dir_all(&build)?;
        util::create_dir_all(&output)?;

        Ok(Self {
            path,
            build,
            output,
        })
    }
}

impl Deref for PatchRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for PatchRoot {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
