use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use log::info;

use crate::hijacker::Hijacker;

use super::{
    function::{RpcFunction, RpcResult},
    skeleton::Skeleton,
};

pub struct SkeletonImpl {
    hijacker: Hijacker,
}

impl SkeletonImpl {
    pub fn new(hijack_map: HashMap<PathBuf, PathBuf>) -> Result<Self> {
        Ok(Self {
            hijacker: Hijacker::new(hijack_map).context("Failed to initialize hijacker")?,
        })
    }
}

impl Skeleton for SkeletonImpl {
    fn enable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("enable hijack: {}", elf_path.display());
            self.hijacker
                .hijack(&elf_path)
                .with_context(|| format!("Failed to hijack \"{}\"", elf_path.display()))
        })
    }

    fn disable_hijack(&self, elf_path: PathBuf) -> RpcResult<()> {
        RpcFunction::call(|| {
            info!("disable hijack: {}", elf_path.display());
            self.hijacker
                .release(&elf_path)
                .with_context(|| format!("Failed to release hajack for \"{}\"", elf_path.display()))
        })
    }
}
