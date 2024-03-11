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

use std::{
    ffi::OsStr,
    fs::File,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use log::{debug, Level};
use syscare_common::{fs, process::Command};

const PATCH_BIN: &str = "patch";
const COMPILER_CMD_ENV: &str = "UPATCH_HIJACKER";
const BUILD_SHELL: &str = "build.sh";

pub struct Project {
    name: String,
    root_dir: PathBuf,
}

impl Project {
    fn patch<P, I, S>(&self, patch_file: P, args: I) -> Result<()>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new(PATCH_BIN)
            .args(args)
            .arg("-i")
            .arg(patch_file.as_ref())
            .current_dir(&self.root_dir)
            .run_with_output()?
            .exit_ok()
    }
}

impl Project {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            name: fs::file_name(&root_dir).to_string_lossy().to_string(),
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn apply_patches<P: AsRef<Path>>(&self, patches: &[P]) -> Result<()> {
        for patch in patches {
            debug!("- Applying patch");
            self.patch(patch, ["-N", "-p1"])
                .with_context(|| format!("Failed to patch {}", patch.as_ref().display()))?;
        }

        Ok(())
    }

    pub fn remove_patches<P: AsRef<Path>>(&self, patches: &[P]) -> Result<()> {
        debug!("- Removing patch");
        for patch in patches.iter().rev() {
            self.patch(patch, ["-R", "-p1"])
                .with_context(|| format!("Failed to unpatch {}", patch.as_ref().display()))?;
        }

        Ok(())
    }

    pub fn test_patches<P: AsRef<Path>>(&self, patches: &[P]) -> Result<()> {
        self.apply_patches(patches)?;
        self.remove_patches(patches)?;

        Ok(())
    }

    pub fn build<S, P>(&self, build_command: S, output_dir: P) -> Result<()>
    where
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        let script_path = output_dir.as_ref().join(BUILD_SHELL);

        let mut script_file = File::create(&script_path)?;
        script_file.write_all(b"#!/bin/bash\n")?;
        script_file.write_all(build_command.as_ref().as_bytes())?;

        Command::new("sh")
            .arg(script_path)
            .env(COMPILER_CMD_ENV, output_dir.as_ref())
            .current_dir(&self.root_dir)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }
}
