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

use std::path::{Path, PathBuf};

use anyhow::Result;
use syscare_common::fs;

pub struct BuildRoot {
    pub path: PathBuf,
    pub original_dir: PathBuf,
    pub patched_dir: PathBuf,
    pub output_dir: PathBuf,
    pub log_file: PathBuf,
}

impl BuildRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let original_dir = path.join("original");
        let patched_dir = path.join("patched");
        let output_dir = path.join("output");
        let log_file = path.join("build.log");

        fs::create_dir_all(&path)?;
        fs::create_dir_all(&original_dir)?;
        fs::create_dir_all(&patched_dir)?;
        fs::create_dir_all(&output_dir)?;

        Ok(Self {
            path,
            original_dir,
            patched_dir,
            output_dir,
            log_file,
        })
    }

    pub fn remove(&self) -> Result<()> {
        fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}
