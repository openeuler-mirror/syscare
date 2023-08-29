use std::os::unix::prelude::MetadataExt;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use log::{debug, info};

mod config;
mod lib;

use config::HijackerConfig;
use lib::HijackLibrary;

pub struct Hijacker {
    lib: HijackLibrary,
    elf_map: HashMap<PathBuf, PathBuf>,
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

        for hijacker in config.0.values() {
            let is_executable_file = hijacker
                .symlink_metadata()
                .map(|m| m.is_file() && (m.mode() & MODE_EXEC_MASK != 0))
                .with_context(|| format!("Failed to read \"{}\" metadata", hijacker.display()))?;
            if !is_executable_file {
                bail!(
                    "Hijack program \"{}\" is not an executable file",
                    hijacker.display()
                );
            }
        }

        Ok(config)
    }
}

impl Hijacker {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let lib = HijackLibrary::new()?;

        debug!("Initializing configuation...");
        let elf_map = Self::initialize_config(config_path)
            .context("Failed to initialize configuration")?
            .0;

        info!("Using elf mapping: {:#?}", elf_map);
        Ok(Self { lib, elf_map })
    }

    fn get_hijacker_path<P: AsRef<Path>>(&self, target: P) -> Result<&Path> {
        let hijacker = self
            .elf_map
            .get(target.as_ref())
            .with_context(|| format!("Cannot find hijacker for \"{}\"", target.as_ref().display()))?
            .as_path();

        Ok(hijacker)
    }

    pub fn hijack<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let target = elf_path.as_ref();
        let hijacker = self.get_hijacker_path(target)?;

        self.lib.hijacker_register(target, hijacker)
    }

    pub fn release<P: AsRef<Path>>(&self, elf_path: P) -> Result<()> {
        let target = elf_path.as_ref();
        let hijacker = self.get_hijacker_path(target)?;

        self.lib.hijacker_unregister(target, hijacker)
    }
}
