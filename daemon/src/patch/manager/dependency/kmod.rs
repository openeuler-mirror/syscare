use std::{
    ffi::OsStr,
    os::unix::prelude::OsStrExt,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use log::{error, info};

const KMOD_NAME: &str = "upatch";
const KMOD_SYS_PATH: &str = "/sys/module/upatch";
const CMD_MODPROBE: &str = "modprobe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleOperation {
    Insert,
    Remove,
}

/// An RAII guard of the `upatch` kernel module.
pub struct KernelModuleGuard;

impl KernelModuleGuard {
    pub fn new() -> Result<Self> {
        if !Self::exists() {
            info!("Loading kernel module...");
            Self::load().context("Failed to load kernel module")?;
        }

        Ok(Self)
    }
}

impl KernelModuleGuard {
    #[inline]
    fn exists() -> bool {
        Path::new(KMOD_SYS_PATH).exists()
    }

    fn exec_module_ops(module_op: ModuleOperation) -> Result<()> {
        let mut cmd = Command::new(CMD_MODPROBE);
        cmd.arg(KMOD_NAME)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        if module_op == ModuleOperation::Remove {
            cmd.arg("--remove");
        }

        let output = cmd.spawn()?.wait_with_output()?;
        if !output.status.success() {
            bail!(OsStr::from_bytes(&output.stderr)
                .to_string_lossy()
                .to_string());
        }

        Ok(())
    }

    #[inline]
    fn load() -> Result<()> {
        Self::exec_module_ops(ModuleOperation::Insert)
    }

    #[inline]
    fn unload() -> Result<()> {
        Self::exec_module_ops(ModuleOperation::Remove)
    }
}

impl Drop for KernelModuleGuard {
    fn drop(&mut self) {
        if Self::exists() {
            info!("Unloading kernel module...");
            if let Err(e) = Self::unload().context("Failed to unload kernel module") {
                error!("{:?}", e);
            }
        }
    }
}

#[test]
fn test() -> Result<()> {
    KernelModuleGuard::new().map(|_| ())
}
