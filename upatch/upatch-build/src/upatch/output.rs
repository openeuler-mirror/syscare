use std::ffi::{OsString, OsStr};
use std::fs;
use std::path::Path;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct OutputConfig {
    binarys: Vec<OsString>,
}

impl OutputConfig {
    pub fn new() -> Self {
        Self { binarys: Vec::new() }
    }

    pub fn push<O: AsRef<OsStr>>(&mut self, binary: O) {
        self.binarys.push(binary.as_ref().to_os_string());
    }

    pub fn create<P: AsRef<Path>>(&self, output_dir: P) -> std::io::Result<()> {
        let config = output_dir.as_ref().join("elf_names");
        fs::write(config, bincode::serialize(&self.binarys).map_err(|e| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Serialize binary name error: {}", e)
        ))?)
    }
}