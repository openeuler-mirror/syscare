use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

use super::package_info::PackageInfo;

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy)]
pub enum PatchType {
    UserPatch,
    KernelPatch,
}

impl std::fmt::Display for PatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PatchFile {
    name:   String,
    path:   PathBuf,
    digest: String,
}

impl std::fmt::Display for PatchFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.name, self.digest))
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PatchInfo {
    name:        String,
    kind:        PatchType,
    arch:        String,
    version:     u32,
    release:     String,
    target:      PackageInfo,
    elf_name:    String,
    license:     String,
    description: String,
    incremental: bool,
    builder:     String,
    patches:     Vec<PatchFile>,
}

impl PatchInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> PatchType {
        self.kind
    }

    pub fn get_arch(&self) -> &str {
        &self.arch
    }

    pub fn get_version(&self) -> u32 {
        self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn get_target(&self) -> &PackageInfo {
        &self.target
    }

    pub fn get_elf_name(&self) -> &str {
        &self.elf_name
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_builder(&self) -> &str {
        &self.builder
    }

    pub fn get_patches(&self) -> &[PatchFile] {
        &self.patches
    }
}

/* Serialize & deserialize */
impl PatchInfo {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        bincode::deserialize_from(std::fs::File::open(path)?).map_err(|e| std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("deserialize path info failed, {}", e)
        ))
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:        {}\n", self.get_name()))?;
        f.write_fmt(format_args!("type:        {}\n", self.get_type()))?;
        f.write_fmt(format_args!("arch:        {}\n", self.get_arch()))?;
        f.write_fmt(format_args!("target:      {}\n", self.get_target().get_simple_name()))?;
        f.write_fmt(format_args!("elf_name:    {}\n", self.get_elf_name()))?;
        f.write_fmt(format_args!("license:     {}\n", self.get_license()))?;
        f.write_fmt(format_args!("version:     {}\n", self.get_version()))?;
        f.write_fmt(format_args!("release:     {}\n", self.get_release()))?;
        f.write_fmt(format_args!("description: {}\n", self.get_description()))?;
        f.write_fmt(format_args!("builder:     {}\n", self.get_builder()))?;
        f.write_str("\npatch list:")?;
        for patch_file in self.get_patches() {
            f.write_fmt(format_args!("\n{}", patch_file))?;
        }

        Ok(())
    }
}
