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

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::Result;
use crate::tool::*;

pub struct BuildRoot {
    cache_dir: PathBuf,
    source_dir: PathBuf,
    patch_dir: PathBuf,
    debuginfo_dir: PathBuf,
    output_dir: PathBuf,
    log_file: PathBuf,
}

impl BuildRoot {
    pub fn new() -> Self {
        Self {
            cache_dir: PathBuf::new(),
            source_dir: PathBuf::new(),
            patch_dir: PathBuf::new(),
            debuginfo_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
            log_file: PathBuf::new(),
        }
    }

    pub fn create_dir<P: AsRef<Path>>(&mut self, build_root: P) -> Result<()> {
        self.cache_dir = build_root.as_ref().to_path_buf();
        self.source_dir = self.cache_dir.join("source");
        self.patch_dir = self.cache_dir.join("patch");
        self.debuginfo_dir = self.cache_dir.join("debug_info");
        self.output_dir = self.cache_dir.join("output");
        self.log_file = self.cache_dir.join("build.log");

        if let Ok(()) = self::check_dir(&self.cache_dir) {
            fs::remove_dir_all(&self.cache_dir)?;
        }

        fs::create_dir_all(&self.cache_dir)?;
        fs::create_dir(&self.source_dir)?;
        fs::create_dir(&self.patch_dir)?;
        fs::create_dir(&self.debuginfo_dir)?;
        fs::create_dir(&self.output_dir)?;
        File::create(&self.log_file)?;
        Ok(())
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn source_dir(&self) -> &Path {
        &self.source_dir
    }

    pub fn patch_dir(&self) -> &Path {
        &self.patch_dir
    }

    pub fn debuginfo_dir(&self) -> &Path {
        &self.debuginfo_dir
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn log_file(&self) -> &Path {
        &self.log_file
    }
}

impl Default for BuildRoot {
    fn default() -> Self {
        Self::new()
    }
}
