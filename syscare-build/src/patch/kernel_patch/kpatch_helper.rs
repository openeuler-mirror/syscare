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
use lazy_static::lazy_static;
use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    fs,
};

pub const VMLINUX_FILE_NAME: &str = "vmlinux";
pub const KPATCH_SUFFIX: &str = "ko";

lazy_static! {
    static ref MAKE: ExternCommand = ExternCommand::new("make");
}

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_defconfig<P: AsRef<Path>>(source_dir: P) -> Result<()> {
        const DEFCONFIG_FILE_NAME: &str = "openeuler_defconfig";

        MAKE.execvp(
            ExternCommandArgs::new()
                .arg("-C")
                .arg(source_dir.as_ref())
                .arg(DEFCONFIG_FILE_NAME),
        )?
        .check_exit_code()
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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};

    use crate::patch::kernel_patch::kpatch_helper::KernelPatchHelper;

    fn create_temp_dir(path: &str) -> Result<PathBuf, std::io::Error> {
        let temp_dir = std::env::temp_dir().join(path);
        if !temp_dir.exists() {
            fs::create_dir(&temp_dir).expect("Failed to create test directory")
        }
        Ok(temp_dir)
    }

    fn create_temp_file<P: AsRef<Path>>(dir: P, filename: &str) -> Result<PathBuf, std::io::Error> {
        let file_path = dir.as_ref().join(filename);
        File::create(&file_path)?;
        Ok(file_path)
    }

    fn cleanup_temp_dir(dir: &Path) {
        if dir.exists() {
            fs::remove_dir_all(dir).unwrap();
        }
    }
    #[test]
    fn test_generate_defconfig() {
        let temp_dir = create_temp_dir("test_defconfig").expect("Failed to create temporary directory");

        let source_dir = temp_dir.join("source_dir");

        fs::create_dir(&source_dir).expect("Failed to create source directory");
        let _defconfig_file = create_temp_file(&source_dir, "openeuler_defconfig")
            .expect("Failed to create defconfig file");

        let makefile_contents = ".PHONY: all openeuler_defconfig clean
openeuler_defconfig:
\techo \"make openeuler_defconfig\"
        ";
        let makefile_path = source_dir.join("Makefile");
        fs::write(&makefile_path, makefile_contents).expect("Failed to write Makefile");

        KernelPatchHelper::generate_defconfig(&source_dir).expect("generate_defconfig failed");

        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_find_kernel_config() {
        let temp_dir = create_temp_dir("test_kernel_config").expect("Failed to create temporary directory");

        let _kernel_config_path = create_temp_file(&temp_dir, ".config")
            .expect("Failed to create kernel config file");

        KernelPatchHelper::find_kernel_config(&temp_dir).expect("Failed to find kernel config");

        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_find_vmlinux() {
        let temp_dir = create_temp_dir("test_vmlinux").expect("Failed to create temporary directory");

        let _vmlinux_path = create_temp_file(&temp_dir, "vmlinux")
            .expect("Failed to create vmlinux file");

        KernelPatchHelper::find_vmlinux(&temp_dir).expect("Failed to find vmlinux file");

        cleanup_temp_dir(&temp_dir);
    }

    #[test]
    fn test_find_kernel_modules() {
        let temp_dir = create_temp_dir("test_kernel_modules").expect("Failed to create temporary directory");

        let module1_path = create_temp_file(&temp_dir, "module1.ko")
            .expect("Failed to create module1.ko file");

        let module2_path = create_temp_file(&temp_dir, "module2.ko")
            .expect("Failed to create module2.ko file");

        let result = KernelPatchHelper::find_kernel_modules(&temp_dir);
        let result_paths = result.unwrap();

        assert_eq!(result_paths.len(), 2);
        assert!(result_paths.contains(&module1_path));
        assert!(result_paths.contains(&module2_path));

        cleanup_temp_dir(&temp_dir);
    }
}

