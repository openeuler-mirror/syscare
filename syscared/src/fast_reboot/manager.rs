// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{error, info};

use syscare_common::{
    fs,
    os::{grub, kernel},
};

lazy_static! {
    static ref BOOT_DIRECTORY: PathBuf = PathBuf::from("/boot");
}

pub enum RebootOption {
    Normal,
    Forced,
}

struct LoadKernelOption {
    name: String,
    kernel: PathBuf,
    initramfs: PathBuf,
}

pub struct KExecManager;

impl KExecManager {
    fn find_kernel(kernel_version: &str) -> Result<LoadKernelOption> {
        info!("Finding kernel {}...", kernel_version);
        let kernel_file_name = format!("vmlinuz-{}", kernel_version);
        let kernel_file = fs::find_file(
            BOOT_DIRECTORY.as_path(),
            kernel_file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )
        .with_context(|| format!("Cannot find kernel {}", kernel_version))?;

        info!("Finding initramfs...");
        let initramfs_file_name = format!("initramfs-{}.img", kernel_version);
        let initramfs_file = fs::find_file(
            BOOT_DIRECTORY.as_path(),
            initramfs_file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )
        .with_context(|| format!("Cannot find kernel {} initramfs", kernel_version))?;

        Ok(LoadKernelOption {
            name: kernel_version.to_owned(),
            kernel: kernel_file,
            initramfs: initramfs_file,
        })
    }

    fn find_kernel_by_grub() -> Result<LoadKernelOption> {
        info!("Parsing grub configuration...");
        let entry = grub::get_boot_entry().context("Failed to read grub boot entry")?;
        let entry_name = entry
            .get_name()
            .to_str()
            .context("Failed to parse grub entry name")?;

        Ok(LoadKernelOption {
            name: entry_name.to_owned(),
            kernel: entry.get_kernel(),
            initramfs: entry.get_initrd(),
        })
    }

    pub fn load_kernel(kernel_version: Option<String>) -> Result<()> {
        let load_option = match kernel_version {
            Some(version) => Self::find_kernel(&version),
            None => Self::find_kernel_by_grub().or_else(|e| {
                error!("{:?}", e);
                let version: &str = kernel::version()
                    .to_str()
                    .context("Failed to parse current kernel version")?;

                Self::find_kernel(version)
            }),
        }?;

        kernel::unload().context("Failed to unload kernel")?;

        let name = load_option.name;
        let kernel = load_option.kernel;
        let initramfs = load_option.initramfs;
        info!("Loading {:?}", name);
        info!("Using kernel: {:?}", kernel);
        info!("Using initrd: {:?}", initramfs);

        kernel::load(&kernel, &initramfs).context("Failed to load kernel")
    }

    pub fn execute(option: RebootOption) -> Result<()> {
        match option {
            RebootOption::Normal => kernel::systemd_exec(),
            RebootOption::Forced => kernel::force_exec(),
        }
        .context("Failed to execute kernel")
    }
}
