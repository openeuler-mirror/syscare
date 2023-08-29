use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::info;

use crate::hijacker::Hijacker;

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::Skeleton,
};

pub struct SkeletonImpl;

impl SkeletonImpl {
    pub fn initialize<P: AsRef<Path>>(config_path: P) -> Result<()> {
        Hijacker::initialize(config_path)
    }
}

impl Skeleton for SkeletonImpl {
    fn enable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Enable hijack: \"{}\"", elf_path.display());
            Hijacker::get_instance()?
                .hijack(&elf_path)
                .with_context(|| format!("Failed to hijack \"{}\"", elf_path.display()))
        })
    }

    fn disable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("Disable hijack: \"{}\"", elf_path.display());
            Hijacker::get_instance()?
                .release(&elf_path)
                .with_context(|| format!("Failed to release hijack for \"{}\"", elf_path.display()))
        })
    }
}
