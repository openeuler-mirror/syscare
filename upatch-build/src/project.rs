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
    ffi::{OsStr, OsString},
    fs::File,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use log::{debug, Level};
use syscare_common::{fs, process::Command};

use crate::{args::Arguments, build_root::BuildRoot};

const PATCH_BIN: &str = "patch";
const COMPILER_CMD_ENV: &str = "UPATCH_HIJACKER";

const PREPARE_SCRIPT_NAME: &str = "prepare.sh";
const BUILD_SCRIPT_NAME: &str = "build.sh";

pub struct Project<'a> {
    name: OsString,
    root_dir: &'a Path,
    build_dir: &'a Path,
    original_dir: &'a Path,
    patched_dir: &'a Path,
    prepare_cmd: &'a str,
    build_cmd: &'a str,
}

impl<'a> Project<'a> {
    pub fn new(args: &'a Arguments, build_root: &'a BuildRoot) -> Self {
        let root_dir = args.source_dir.as_path();
        let build_dir = build_root.output_dir.as_path();
        let original_dir = build_root.original_dir.as_path();
        let patched_dir = build_root.patched_dir.as_path();

        let name = fs::file_name(&root_dir);
        let prepare_cmd = args.prepare_cmd.as_str();
        let build_cmd = args.build_cmd.as_str();

        Self {
            name,
            root_dir,
            build_dir,
            original_dir,
            patched_dir,
            prepare_cmd,
            build_cmd,
        }
    }
}

impl Project<'_> {
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

    fn create_script<S, T>(&self, script_name: S, command: T) -> Result<PathBuf>
    where
        S: AsRef<OsStr>,
        T: AsRef<OsStr>,
    {
        let script = self.build_dir.join(script_name.as_ref());

        let mut script_file = File::create(&script)?;
        script_file.write_all(b"#!/bin/bash\n")?;
        script_file.write_all(command.as_ref().as_bytes())?;
        drop(script_file);

        Ok(script)
    }

    fn exec_command<S, T>(&self, script_name: S, command: T) -> Result<()>
    where
        S: AsRef<OsStr>,
        T: AsRef<OsStr>,
    {
        let script = self.create_script(script_name, command)?;

        Command::new("sh")
            .arg(script)
            .current_dir(&self.root_dir)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }

    fn exec_build_command<S, T, P>(&self, script_name: S, command: T, object_dir: P) -> Result<()>
    where
        S: AsRef<OsStr>,
        T: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        if command.as_ref().is_empty() {
            return Ok(());
        }
        let script = self.create_script(script_name, command)?;

        Command::new("sh")
            .arg(script)
            .env(COMPILER_CMD_ENV, object_dir.as_ref())
            .current_dir(&self.root_dir)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }
}

impl Project<'_> {
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

    pub fn prepare(&self) -> Result<()> {
        self.exec_command(PREPARE_SCRIPT_NAME, self.prepare_cmd)
    }

    pub fn build(&self) -> Result<()> {
        self.exec_build_command(BUILD_SCRIPT_NAME, self.build_cmd, self.original_dir)
    }

    pub fn rebuild(&self) -> Result<()> {
        self.exec_build_command(BUILD_SCRIPT_NAME, self.build_cmd, self.patched_dir)
    }
}

impl std::fmt::Display for Project<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.to_string_lossy())
    }
}
