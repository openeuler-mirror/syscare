use std::{
    ffi::OsStr,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};
use log::{debug, error};

const KMOD_NAME: &str = "upatch_hijacker";
const KMOD_SYS_DIR: &str = "/sys/module";
const CMD_MODPROBE: &str = "modprobe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleOperation {
    Insert,
    Remove,
}

/// An RAII guard of the hijack kernel module implementation.
pub struct KernelModuleGuard {
    name: String,
    sys_path: PathBuf,
}

impl KernelModuleGuard {
    pub fn new() -> Result<Self> {
        let instance: KernelModuleGuard = Self {
            name: KMOD_NAME.to_string(),
            sys_path: Path::new(KMOD_SYS_DIR).join(KMOD_NAME),
        };
        instance
            .load()
            .with_context(|| format!("Failed to load module \"{}\"", instance))?;

        Ok(instance)
    }
}

impl KernelModuleGuard {
    fn exec_module_ops(&self, module_op: ModuleOperation) -> Result<()> {
        let mut cmd = Command::new(CMD_MODPROBE);
        cmd.arg(&self.name).stderr(Stdio::piped());

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

    fn exists(&self) -> bool {
        self.sys_path.exists()
    }

    fn load(&self) -> Result<()> {
        if !self.exists() {
            debug!("Loading kernel module \"{}\"...", self);
            self.exec_module_ops(ModuleOperation::Insert)?;
        }
        Ok(())
    }

    fn unload(&self) -> Result<()> {
        if self.exists() {
            debug!("Unloading kernel module \"{}\"...", self);
            self.exec_module_ops(ModuleOperation::Remove)?;
        }
        Ok(())
    }
}

impl Drop for KernelModuleGuard {
    fn drop(&mut self) {
        if let Err(e) = self
            .unload()
            .with_context(|| format!("Failed to unload module \"{}\"", self))
        {
            error!("{:?}", e);
        }
    }
}

impl std::fmt::Display for KernelModuleGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[test]
fn test() -> Result<()> {
    KernelModuleGuard::new().map(|_| ())
}
