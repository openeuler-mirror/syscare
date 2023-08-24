use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

const CC_BINARY: &str = "/usr/bin/cc";
const CXX_BINARY: &str = "/usr/bin/c++";
const GCC_BINARY: &str = "/usr/bin/gcc";
const GXX_BINARY: &str = "/usr/bin/g++";
const AS_BINARY: &str = "/usr/bin/as";

const CC_HIJACKER: &str = "/usr/libexec/syscare/cc-hijacker";
const CXX_HIJACKER: &str = "/usr/libexec/syscare/c++-hijacker";
const GCC_HIJACKER: &str = "/usr/libexec/syscare/gcc-hijacker";
const GXX_HIJACKER: &str = "/usr/libexec/syscare/g++-hijacker";
const AS_HIJACKER: &str = "/usr/libexec/syscare/as-hijacker";

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
                (PathBuf::from(CC_BINARY), PathBuf::from(CC_HIJACKER)),
                (PathBuf::from(CXX_BINARY), PathBuf::from(CXX_HIJACKER)),
                (PathBuf::from(GCC_BINARY), PathBuf::from(GCC_HIJACKER)),
                (PathBuf::from(GXX_BINARY), PathBuf::from(GXX_HIJACKER)),
                (PathBuf::from(AS_BINARY), PathBuf::from(AS_HIJACKER)),
            ]),
        }
    }
}
