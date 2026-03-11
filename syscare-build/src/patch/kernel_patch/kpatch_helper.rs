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

use anyhow::{Context, Result};
use log::Level;
use syscare_common::{fs, os::cpu::arch, process::Command};

pub const VMLINUX_FILE_NAME: &str = "vmlinux";
pub const KPATCH_SUFFIX: &str = "ko";

const MAKE_BIN: &str = "make";

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_config_file<P, S>(source_dir: P, name: S) -> Result<()>
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        Command::new(MAKE_BIN)
            .arg("-C")
            .arg(source_dir.as_ref())
            .arg(name)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }

    pub fn update_kernel_config<P, Q>(kernel_source_dir: P, config_file: Q) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let arch = arch();
        let arch_name = match arch.to_str().unwrap_or_default() {
            "x86_64" => "x86",
            "aarch64" => "arm64",
            name => name,
        };
        let new_config = kernel_source_dir.as_ref().join(".config");

        // Clean old files
        Command::new(MAKE_BIN)
            .arg("-C")
            .arg(kernel_source_dir.as_ref())
            .arg("mrproper")
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()?;

        //Write current kernel config
        let curr_config = config_file.as_ref();
        if curr_config != new_config {
            fs::copy(curr_config, &new_config)
                .with_context(|| format!("Failed to write '{}'", new_config.display()))?;
        }

        // Generate suitable config file
        Command::new(MAKE_BIN)
            .arg("-C")
            .arg(kernel_source_dir.as_ref())
            .arg(format!("ARCH={}", arch_name))
            .arg("olddefconfig")
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()?;

        Ok(new_config)
    }

    pub fn find_vmlinux<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        fs::find_file(
            directory,
            VMLINUX_FILE_NAME,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )
    }

    pub fn find_kernel_modules<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        fs::list_files_by_ext(
            directory,
            KPATCH_SUFFIX,
            fs::TraverseOptions { recursive: true },
        )
    }
}
