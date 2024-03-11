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
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use syscare_common::process::Command;

const TAR_BIN: &str = "tar";

pub struct TarPackage {
    path: PathBuf,
}

impl TarPackage {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn compress<P, S>(&self, root_dir: P, target: S) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        Command::new(TAR_BIN)
            .arg("-czf")
            .arg(self.path.as_path())
            .arg("-C")
            .arg(root_dir.as_ref())
            .arg(target)
            .arg("--restrict")
            .run()?
            .exit_ok()
    }

    pub fn decompress<P>(&self, output_dir: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        if !self.path.is_file() {
            bail!("File {} is not exist", self.path.display());
        }

        Command::new(TAR_BIN)
            .arg("-xf")
            .arg(self.path.as_path())
            .arg("-C")
            .arg(output_dir.as_ref())
            .arg("--no-same-owner")
            .arg("--no-same-permissions")
            .arg("--restrict")
            .run()?
            .exit_ok()
    }
}
