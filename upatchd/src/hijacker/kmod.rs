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
    ffi::{CString, OsString},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use log::{error, info};
use nix::kmod;
use syscare_common::{fs, os};

const KMOD_SYS_PATH: &str = "/sys/module";

/// An RAII guard of the kernel module.
pub struct HijackerKmodGuard {
    kmod_name: String,
    kmod_path: PathBuf,
    sys_path: PathBuf,
}

impl HijackerKmodGuard {
    pub fn new<S: AsRef<str>, P: AsRef<Path>>(name: S, kmod_path: P) -> Result<Self> {
        let instance = Self {
            kmod_name: name.as_ref().to_string(),
            kmod_path: kmod_path.as_ref().to_path_buf(),
            sys_path: Path::new(KMOD_SYS_PATH).join(name.as_ref()),
        };
        instance.selinux_relabel_kmod()?;
        instance.install_kmod()?;

        Ok(instance)
    }
}

impl HijackerKmodGuard {
    fn selinux_relabel_kmod(&self) -> Result<()> {
        const KMOD_SECURITY_TYPE: &str = "modules_object_t";

        if os::selinux::get_status()? != os::selinux::Status::Enforcing {
            return Ok(());
        }

        info!("Relabeling kernel module '{}'...", self.kmod_name);
        let mut sec_context = os::selinux::get_security_context(&self.kmod_path)?;
        if sec_context.kind != KMOD_SECURITY_TYPE {
            sec_context.kind = OsString::from(KMOD_SECURITY_TYPE);
            os::selinux::set_security_context(&self.kmod_path, sec_context)?;
        }

        Ok(())
    }

    fn install_kmod(&self) -> Result<()> {
        if self.sys_path.exists() {
            return Ok(());
        }

        info!("Installing kernel module '{}'...", self.kmod_name);
        let ko_file = fs::open_file(&self.kmod_path)?;
        kmod::finit_module(
            &ko_file,
            CString::new("")?.as_c_str(),
            kmod::ModuleInitFlags::MODULE_INIT_IGNORE_VERMAGIC,
        )
        .with_context(|| format!("Failed to install kernel module '{}'", self.kmod_name))
    }

    fn remove_kmod(&self) -> Result<()> {
        if !self.sys_path.exists() {
            return Ok(());
        }

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
        if let Err(e) = self.remove_kmod() {
            error!("{:?}", e);
        }
    }
}
