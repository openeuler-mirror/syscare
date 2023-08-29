use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use log::{error, info};

const EBPF_BIN_PATH: &str = "/usr/libexec/syscare/upatch_hijacker";
const EBPF_SOCKET_PATH: &str = "/var/run/upatch-hijacker";
const EBPF_WAIT_TIMEOUT: u64 = 100;
const EBPF_WAIT_MAX_RETRY: u64 = 2;

/// An RAII guard of the `upatch_hijack` eBPF program.
pub struct EbpfProgramGuard {
    process: Option<Child>,
}

impl EbpfProgramGuard {
    pub fn new() -> Result<Self> {
        let mut instance = Self { process: None };
        if !Self::exists() {
            info!("Starting eBPF program...");
            instance.start().context("Failed to start eBPF program")?;
        }

        Ok(instance)
    }
}

impl EbpfProgramGuard {
    #[inline]
    fn exists() -> bool {
        Path::new(EBPF_SOCKET_PATH).exists()
    }

    fn start(&mut self) -> Result<()> {
        let process = &mut self.process;
        if process.is_none() {
            let mut child = Command::new(EBPF_BIN_PATH)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut wait_retry = 0;
            loop {
                let wait_result = child.try_wait().context("Failed to wait eBPF program")?;
                match wait_result {
                    Some(exit_status) => {
                        if exit_status.code().unwrap_or_default() != 0 {
                            bail!("Hijacker eBPF program exited unexpectedly");
                        }
                    }
                    None => {
                        if wait_retry >= EBPF_WAIT_MAX_RETRY {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(EBPF_WAIT_TIMEOUT));
                        wait_retry += 1;
                    }
                }
            }

            let _ = process.insert(child);
        }

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill()?
        }

        Ok(())
    }
}

impl Drop for EbpfProgramGuard {
    fn drop(&mut self) {
        if Self::exists() {
            info!("Stopping eBPF program...");
            if let Err(e) = self.stop().context("Failed to stop eBPF program") {
                error!("{:?}", e)
            }
        }
    }
}

#[test]
fn test() -> Result<()> {
    EbpfProgramGuard::new().map(|_| ())
}
