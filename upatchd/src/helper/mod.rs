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

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use log::{debug, error, info};

use syscare_common::{fs::MappedFile, os};

mod config;
mod elf_resolver;
mod ioctl;
mod kmod;

pub use config::UpatchHelperConfig;
use elf_resolver::ElfResolver;
use ioctl::UpatchHelperIoctl;
use kmod::UpatchHelperKmodGuard;

const KMOD_NAME: &str = "upatch_helper";
const KMOD_DEV_PATH: &str = "/dev/upatch-helper";
const KMOD_PATH: &str = "/usr/libexec/syscare/upatch_helper.ko";

const TARGET_SYMBOL_NAME: &str = "execve";

pub struct UpatchHelper {
    config: UpatchHelperConfig,
    ioctl: UpatchHelperIoctl,
    _kmod: UpatchHelperKmodGuard, // need to ensure this drops last
}

impl UpatchHelper {
    fn find_symbol_addr(symbol_name: &str) -> Result<(PathBuf, u64)> {
        let exec_file = MappedFile::open(os::process::path())?;
        let exec_resolver = ElfResolver::new(exec_file.as_bytes())?;

        for lib_path in exec_resolver.dependencies()? {
            let lib_file = MappedFile::open(&lib_path)?;
            let lib_resolver = ElfResolver::new(lib_file.as_bytes())?;

            if let Ok(Some(addr)) = lib_resolver.find_symbol_addr(symbol_name) {
                return Ok((lib_path, addr));
            }
        }

        bail!("Failed to find symbol '{}'", symbol_name);
    }
}

impl UpatchHelper {
    pub fn new(config: UpatchHelperConfig) -> Result<Self> {
        debug!("Initializing upatch kernel module...");
        let kmod = UpatchHelperKmodGuard::new(KMOD_NAME, KMOD_PATH)?;

        debug!("Initializing upatch ioctl channel...");
        let ioctl = UpatchHelperIoctl::new(KMOD_DEV_PATH)?;

        debug!("Initializing upatch hooks...");
        let (lib_path, offset) = Self::find_symbol_addr(TARGET_SYMBOL_NAME)?;
        info!(
            "Hooking library: {}, offset: {:#x}",
            lib_path.display(),
            offset
        );
        ioctl.enable_hook(lib_path, offset)?;

        Ok(Self {
            config,
            ioctl,
            _kmod: kmod,
        })
    }

    pub fn register_hooker<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let exec_path = elf_path.as_ref();
        let jump_path = self.jump_path(exec_path)?;

        self.ioctl.register_hooker(exec_path, jump_path)
    }

    pub fn unregister_hooker<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let exec_path = elf_path.as_ref();
        let jump_path = self.jump_path(exec_path)?;

        self.ioctl.unregister_hooker(exec_path, jump_path)
    }
}

impl UpatchHelper {
    fn jump_path<P: AsRef<Path>>(&self, exec_path: P) -> Result<&Path> {
        let jump_path = self
            .config
            .mapping
            .get(exec_path.as_ref())
            .with_context(|| {
                format!(
                    "Cannot find hook program for {}",
                    exec_path.as_ref().display()
                )
            })?
            .as_path();

        Ok(jump_path)
    }
}

impl Drop for UpatchHelper {
    fn drop(&mut self) {
        if let Err(e) = self.ioctl.disable_hook() {
            error!("{:?}", e);
        }
    }
}
