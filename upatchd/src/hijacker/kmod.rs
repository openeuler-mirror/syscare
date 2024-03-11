// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
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
    ffi::CString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use log::{error, info};
use nix::kmod;
use syscare_common::fs;

const KMOD_SYS_PATH: &str = "/sys/module";

/// An RAII guard of the kernel module.
pub struct HijackerKmodGuard {
    kmod_name: String,
    sys_path: PathBuf,
}

impl HijackerKmodGuard {
    pub fn new<S: AsRef<str>, P: AsRef<Path>>(name: S, path: P) -> Result<Self> {
        let kmod_name = name.as_ref().to_string();
        let sys_path = Path::new(KMOD_SYS_PATH).join(name.as_ref());
        let kmod_path = path.as_ref().to_path_buf();

        let instance: HijackerKmodGuard = Self {
            kmod_name,
            sys_path,
        };
        if !instance.is_installed() {
            instance.install(kmod_path)?;
        }
        Ok(instance)
    }
}

impl HijackerKmodGuard {
    #[inline]
    fn is_installed(&self) -> bool {
        self.sys_path.exists()
    }

    fn install<P: AsRef<Path>>(&self, kmod_path: P) -> Result<()> {
        info!("Installing kernel module '{}'...", self.kmod_name);
        let ko_file = fs::open_file(kmod_path)?;
        kmod::finit_module(
            &ko_file,
            CString::new("")?.as_c_str(),
            kmod::ModuleInitFlags::MODULE_INIT_IGNORE_VERMAGIC,
        )
        .with_context(|| format!("Failed to install kernel module '{}'", self.kmod_name))
    }

    fn remove(&self) -> Result<()> {
        info!("Removing kernel module '{}'...", self.kmod_name);
        kmod::delete_module(
            CString::new(self.kmod_name.as_str())?.as_c_str(),
            kmod::DeleteModuleFlags::O_NONBLOCK,
        )
        .with_context(|| format!("Failed to remove kernel module '{}'", self.kmod_name))
    }
}

impl Drop for HijackerKmodGuard {
    fn drop(&mut self) {
        if self.is_installed() {
            if let Err(e) = self.remove() {
                error!("{:?}", e);
            }
        }
    }
}
