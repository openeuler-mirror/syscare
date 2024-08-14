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
    env,
    ffi::{OsStr, OsString},
    fs::File,
    io::Write,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use log::{debug, Level};
use which::which;

use crate::{args::Arguments, build_root::BuildRoot, compiler::CompilerInfo, dwarf::ProducerType};
use syscare_common::{concat_os, fs, process::Command};

const PATCH_BIN: &str = "patch";
const UPATCH_HELPER_BIN: &str = "upatch-helper";
const UPATCH_HELPER_CC_BIN: &str = "upatch-cc";
const UPATCH_HELPER_CXX_BIN: &str = "upatch-c++";

const PREPARE_SCRIPT_NAME: &str = "prepare.sh";
const BUILD_SCRIPT_NAME: &str = "build.sh";
const CLEAN_SCRIPT_NAME: &str = "clean.sh";

const PATH_ENV: &str = "PATH";
const CC_ENV: &str = "CC";
const CXX_ENV: &str = "CXX";

const UPATCH_CC_ENV: &str = "UPATCH_HELPER_CC";
const UPATCH_CXX_ENV: &str = "UPATCH_HELPER_CXX";

pub struct Project<'a> {
    name: OsString,
    build_root: &'a BuildRoot,
    source_dir: &'a Path,
    prepare_cmd: &'a OsStr,
    build_cmd: &'a OsStr,
    clean_cmd: &'a OsStr,
    patches: &'a [PathBuf],
}

impl<'a> Project<'a> {
    pub fn new(
        args: &'a Arguments,
        build_root: &'a BuildRoot,
        compiler_map: &'a IndexMap<ProducerType, CompilerInfo>,
    ) -> Result<Self> {
        let path_env = env::var_os(PATH_ENV)
            .with_context(|| format!("Cannot read environment variable '{}'", PATH_ENV))?;
        let upatch_helper = which(UPATCH_HELPER_BIN)
            .with_context(|| format!("Cannot find component '{}'", UPATCH_HELPER_BIN))?;

        for (producer_type, compiler_info) in compiler_map {
            let compiler_bin = compiler_info.binary.as_path();
            let compiler_name = compiler_bin
                .file_name()
                .context("Failed to parse compiler name")?;

            match producer_type {
                ProducerType::C => env::set_var(UPATCH_CC_ENV, compiler_bin),
                ProducerType::Cxx => env::set_var(UPATCH_CXX_ENV, compiler_bin),
                _ => {}
            }
            fs::soft_link(&upatch_helper, build_root.bin_dir.join(compiler_name))?;
        }

        env::set_var(PATH_ENV, concat_os!(&build_root.bin_dir, ":", path_env));
        env::set_var(CC_ENV, UPATCH_HELPER_CC_BIN);
        env::set_var(CXX_ENV, UPATCH_HELPER_CXX_BIN);

        Ok(Self {
            name: args
                .source_dir
                .file_name()
                .context("Failed to parse project name")?
                .to_os_string(),
            build_root,
            source_dir: args.source_dir.as_path(),
            prepare_cmd: args.prepare_cmd.as_os_str(),
            build_cmd: args.build_cmd.as_os_str(),
            clean_cmd: args.clean_cmd.as_os_str(),
            patches: args.patch.as_slice(),
        })
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
            .current_dir(self.source_dir)
            .run_with_output()?
            .exit_ok()
    }

    fn create_script<S, T>(&self, script_name: S, command: T) -> Result<PathBuf>
    where
        S: AsRef<OsStr>,
        T: AsRef<OsStr>,
    {
        let script = self.build_root.script_dir.join(script_name.as_ref());

        let mut script_file = File::create(&script)?;
        script_file.write_all(b"#!/bin/bash\n")?;
        script_file.write_all(command.as_ref().as_bytes())?;

        Ok(script)
    }

    fn exec_command<S, T>(&self, script_name: S, command: T) -> Result<()>
    where
        S: AsRef<OsStr>,
        T: AsRef<OsStr>,
    {
        if command.as_ref().is_empty() {
            return Ok(());
        }
        let script = self.create_script(script_name, command)?;
        Command::new("sh")
            .arg(script)
            .current_dir(self.source_dir)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }
}

impl Project<'_> {
    pub fn apply_patches(&self) -> Result<()> {
        for patch in self.patches.iter() {
            debug!("* {}", patch.display());
            self.patch(patch, ["-N", "-p1"])
                .with_context(|| format!("Failed to patch {}", patch.display()))?;
        }

        Ok(())
    }

    pub fn remove_patches(&self) -> Result<()> {
        for patch in self.patches.iter().rev() {
            self.patch(patch, ["-R", "-p1"])
                .with_context(|| format!("Failed to unpatch {}", patch.display()))?;
        }

        Ok(())
    }

    pub fn test_patches(&self) -> Result<()> {
        self.apply_patches()?;
        self.remove_patches()?;

        Ok(())
    }

    pub fn prepare(&self) -> Result<()> {
        self.exec_command(PREPARE_SCRIPT_NAME, self.prepare_cmd)
    }

    pub fn build(&self) -> Result<()> {
        self.exec_command(BUILD_SCRIPT_NAME, self.build_cmd)
    }

    pub fn clean(&self) -> Result<()> {
        self.exec_command(CLEAN_SCRIPT_NAME, self.clean_cmd)
    }
}

impl std::fmt::Display for Project<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.to_string_lossy())
    }
}

impl Drop for Project<'_> {
    fn drop(&mut self) {
        self.remove_patches().ok();
    }
}
