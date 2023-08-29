use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
};

use anyhow::{Context, Result};
use log::{debug, error};

const EBPF_BIN_PATH: &str = "/usr/libexec/syscare/upatch_hijacker";
const EBPF_SOCKET_PATH: &str = "/var/run/upatch-hijacker";

/// An RAII guard of the hijack ebpf implementation.
pub struct EbpfProgramGuard {
    elf_path: PathBuf,
    process: Option<Child>,
}

impl EbpfProgramGuard {
    pub fn new() -> Result<Self> {
        let mut instance = Self {
            elf_path: PathBuf::from(EBPF_BIN_PATH),
            process: None,
        };
        instance.start().context("Failed to start hijacker ebpf")?;

        Ok(instance)
    }
}

impl EbpfProgramGuard {
    fn exists(&self) -> bool {
        Path::new(EBPF_SOCKET_PATH).exists()
    }

    fn start(&mut self) -> Result<()> {
        if !self.exists() {
            debug!("Starting ebpf program \"{}\"...", EBPF_BIN_PATH);
            let process = &mut self.process;
            if process.is_none() {
                let child = Command::new(&self.elf_path).spawn()?;
                let _ = process.insert(child);
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if self.exists() {
            debug!("Stopping ebpf program \"{}\"...", EBPF_BIN_PATH);
            if let Some(mut child) = self.process.take() {
                child.kill()?
            }
        }
        Ok(())
    }
}

impl Drop for EbpfProgramGuard {
    fn drop(&mut self) {
        if let Err(e) = self.stop().context("Failed to stop hijacker ebpf") {
            error!("{:?}", e)
        }
    }
}

#[test]
fn test() -> Result<()> {
    EbpfProgramGuard::new().map(|_| ())
}
