use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

const GNU_GCC: &str = "/usr/bin/gcc";
const GNU_GXX: &str = "/usr/bin/g++";
const GNU_AS: &str = "/usr/bin/as";

const GNU_COMPILER_HIJACKER: &str = "/usr/libexec/syscare/upatch-gnu-compiler-hijacker";
const GNU_ASSEMBLER_HIJACKER: &str = "/usr/libexec/syscare/upatch-gnu-as-hijacker";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub elf_map: HashMap<PathBuf, PathBuf>,
}

impl Config {
    pub fn parse_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_path = path.as_ref();
        let config_file = File::open(config_path)
            .with_context(|| format!("Failed to open config \"{}\"", config_path.display()))?;
        let instance: Self = serde_yaml::from_reader(config_file)
            .map_err(|_| anyhow!("Failed to parse config \"{}\"", config_path.display()))?;

        Ok(instance)
    }

    pub fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config_path = path.as_ref();
        let config_file = File::create(config_path)
            .with_context(|| format!("Failed to create config \"{}\"", config_path.display()))?;
        serde_yaml::to_writer(config_file, self)
            .map_err(|_| anyhow!("Failed to write config \"{}\"", config_path.display()))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            elf_map: HashMap::from([
                (PathBuf::from(GNU_GCC), PathBuf::from(GNU_COMPILER_HIJACKER)),
                (PathBuf::from(GNU_GXX), PathBuf::from(GNU_COMPILER_HIJACKER)),
                (PathBuf::from(GNU_AS), PathBuf::from(GNU_ASSEMBLER_HIJACKER)),
            ]),
        }
    }
}
