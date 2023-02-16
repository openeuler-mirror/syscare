use std::ffi::OsString;
use std::path::PathBuf;
use std::collections::HashMap;

use log::log;
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
    version:     u32,
    release:     String,
    arch:        String,
    kind:        PatchType,
    target:      PackageInfo,
    target_elfs: HashMap<OsString, PathBuf>,
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

    pub fn get_version(&self) -> u32 {
        self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn get_arch(&self) -> &str {
        &self.arch
    }

    pub fn get_type(&self) -> PatchType {
        self.kind
    }

    pub fn get_target(&self) -> &PackageInfo {
        &self.target
    }

    pub fn get_target_elfs(&self) -> &HashMap<OsString, PathBuf> {
        &self.target_elfs
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

impl PatchInfo {
    fn get_target_elfs_str(&self) -> String {
        const PATCH_FLAG_NONE: &str = "(none)";

        let elf_list = self.get_target_elfs();
        if elf_list.is_empty() {
            return PATCH_FLAG_NONE.to_owned();
        }

        let mut str = String::new();
        for (elf_name, _) in elf_list.into_iter() {
            str.push_str(&format!("{}, ", elf_name.to_string_lossy()));
        }
        str.pop();
        str.pop();
        str
    }

    pub fn print_log(&self, level: log::Level) {
        log!(level, "name:        {}", self.get_name());
        log!(level, "version:     {}", self.get_version());
        log!(level, "release:     {}", self.get_release());
        log!(level, "arch:        {}", self.get_arch());
        log!(level, "type:        {}", self.get_type());
        log!(level, "target:      {}", self.get_target().get_name());
        log!(level, "target_elfs: {}", self.get_target_elfs_str());
        log!(level, "license:     {}", self.get_license());
        log!(level, "description: {}", self.get_description());
        log!(level, "builder:     {}", self.get_builder());
        log!(level, "");
        log!(level, "patch list:");
        for patch_file in self.get_patches() {
            log!(level, "{} {}", patch_file.name, patch_file.digest);
        }
    }
}
