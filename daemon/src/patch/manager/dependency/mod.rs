use anyhow::Result;

mod kmod;

use kmod::KernelModuleGuard;

pub enum PatchManagerDependency {
    KernelModule(KernelModuleGuard),
}

impl PatchManagerDependency {
    pub fn new() -> Result<Self> {
        Ok(Self::KernelModule(KernelModuleGuard::new()?))
    }
}
