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

use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use log::{debug, error, info};

use syscare_common::{fs::MappedFile, os};

mod config;
mod elf_resolver;
mod ioctl;
mod kmod;

use config::HijackerConfig;
use elf_resolver::ElfResolver;
use ioctl::HijackerIoctl;
use kmod::HijackerKmodGuard;

const KMOD_NAME: &str = "upatch_hijacker";
const KMOD_DEV_PATH: &str = "/dev/upatch-hijacker";
const KMOD_FILE_PATH: &str = "/usr/libexec/syscare/upatch_hijacker.ko";

const HIJACK_SYMBOL_NAME: &str = "execve";

pub struct Hijacker {
    config: HijackerConfig,
    ioctl: HijackerIoctl,
    _kmod: HijackerKmodGuard, // need to ensure this drops last
}

impl Hijacker {
    fn initialize_config<P: AsRef<Path>>(config_path: P) -> Result<HijackerConfig> {
        const MODE_EXEC_MASK: u32 = 0o111;

        let config = match config_path.as_ref().exists() {
            true => HijackerConfig::parse_from(config_path)?,
            false => {
                info!("Generating default configuration...");
                let config = HijackerConfig::default();
                config.write_to(config_path)?;

                config
            }
        };

        for hijacker in config.values() {
            let is_executable_file = hijacker
                .symlink_metadata()
                .map(|m| m.is_file() && (m.mode() & MODE_EXEC_MASK != 0))
                .with_context(|| format!("Failed to read {} metadata", hijacker.display()))?;
            if !is_executable_file {
                bail!(
                    "Hijack program {} is not an executable file",
                    hijacker.display()
                );
            }
        }

        Ok(config)
    }

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

impl Hijacker {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        debug!("Initializing hijacker configuation...");
        let config = Self::initialize_config(config_path)
            .context("Failed to initialize hijacker configuration")?;
        info!("Using elf mapping: {}", config);

        debug!("Initializing hijacker kernel module...");
        let kmod_name = KMOD_NAME.to_string();
        let kmod_path = KMOD_FILE_PATH.to_string();
        let kmod = HijackerKmodGuard::new(kmod_name, kmod_path)?;

        debug!("Initializing hijacker ioctl channel...");
        let ioctl = HijackerIoctl::new(KMOD_DEV_PATH)?;

        debug!("Initializing hijacker hooks...");
        let (lib_path, offset) = Self::find_symbol_addr(HIJACK_SYMBOL_NAME)?;
        info!(
            "Hooking library: {}, offset: {:#x}",
            lib_path.display(),
            offset
        );
        ioctl.enable_hijacker(lib_path, offset)?;

        Ok(Self {
            _kmod: kmod,
            ioctl,
            config,
        })
    }
}

impl Hijacker {
    fn get_hijacker<P: AsRef<Path>>(&self, exec_path: P) -> Result<&Path> {
        let hijacker = self
            .config
            .get(exec_path.as_ref())
            .with_context(|| format!("Cannot find hijacker for {}", exec_path.as_ref().display()))?
            .as_path();

        Ok(hijacker)
    }

    pub fn register<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let exec_path = elf_path.as_ref();
        let jump_path = self.get_hijacker(exec_path)?;

        self.ioctl.register_hijacker(exec_path, jump_path)
    }

    pub fn unregister<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let exec_path = elf_path.as_ref();
        let jump_path = self.get_hijacker(exec_path)?;

        self.ioctl.unregister_hijacker(exec_path, jump_path)
    }
}

impl Drop for Hijacker {
    fn drop(&mut self) {
        if let Err(e) = self.ioctl.disable_hijacker() {
            error!("{:?}", e);
        }
    }
}
