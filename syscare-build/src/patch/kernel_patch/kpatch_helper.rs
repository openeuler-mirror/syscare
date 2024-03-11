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
use log::Level;
use syscare_common::{fs, process::Command};

pub const VMLINUX_FILE_NAME: &str = "vmlinux";
pub const KPATCH_SUFFIX: &str = "ko";

const MAKE_BIN: &str = "make";

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_defconfig<P: AsRef<Path>>(source_dir: P) -> Result<()> {
        const DEFCONFIG_FILE_NAME: &str = "openeuler_defconfig";

        Command::new(MAKE_BIN)
            .arg("-C")
            .arg(source_dir.as_ref())
            .arg(DEFCONFIG_FILE_NAME)
            .stdout(Level::Debug)
            .run_with_output()?
            .exit_ok()
    }

    pub fn find_kernel_config<P: AsRef<Path>>(directory: P) -> Result<PathBuf> {
        const KERNEL_CONFIG_FILE_NAME: &str = ".config";

        Ok(fs::find_file(
            directory,
            KERNEL_CONFIG_FILE_NAME,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )?)
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
