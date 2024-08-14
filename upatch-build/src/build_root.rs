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
    pub bin_dir: PathBuf,
    pub script_dir: PathBuf,
    pub build_dir: PathBuf,
    pub original_dir: PathBuf,
    pub patched_dir: PathBuf,
    pub log_file: PathBuf,
}

impl BuildRoot {
    pub fn new<P: AsRef<Path>>(directory: P) -> Result<Self> {
        let path = directory.as_ref().to_path_buf();
        let bin_dir = path.join("bin");
        let script_dir = path.join("script");
        let build_dir = path.join("build");
        let original_dir = path.join("original");
        let patched_dir = path.join("patched");
        let log_file = path.join("build.log");

        fs::create_dir_all(&path)?;
        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&script_dir)?;
        fs::create_dir_all(&build_dir)?;
        fs::create_dir_all(&original_dir)?;
        fs::create_dir_all(&patched_dir)?;

        Ok(Self {
            path,
            bin_dir,
            script_dir,
            build_dir,
            original_dir,
            patched_dir,
            log_file,
        })
    }

    pub fn remove(&self) -> Result<()> {
        fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}
