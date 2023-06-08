use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use log::log;
use serde::{Deserialize, Serialize};

use super::package_info::PackageInfo;

/*
 * In order to solve PatchInfo binary compatibility issue,
 * we use this version string to perform compatibility check
 * before PatchInfo deserialization.
 * Therefore, whenever the PatchInfo is modified (including PackageInfo),
 * this should be updated and keep sync with patch builder.
 */
const PATCH_INFO_MAGIC: &str = "2B96A33EC26809077";

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum PatchType {
    UserPatch,
    KernelPatch,
}

impl std::fmt::Display for PatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchFile {
    pub name: OsString,
    pub path: PathBuf,
    pub digest: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchInfo {
    pub uuid: String,
    pub name: String,
    pub version: String,
    pub release: u32,
    pub arch: String,
    pub kind: PatchType,
    pub digest: String,
    pub target: PackageInfo,
    pub target_elfs: HashMap<OsString, PathBuf>, // (elf_name, elf_path)
    pub license: String,
    pub description: String,
    pub patches: Vec<PatchFile>,
}

impl PatchInfo {
    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.name, self.version, self.release, self.arch
        )
    }

    pub fn version() -> &'static str {
        PATCH_INFO_MAGIC
    }
}

impl PatchInfo {
    pub fn print_log(&self, level: log::Level) {
        const PATCH_FLAG_NONE: &str = "(none)";

        let target_elfs = match self.target_elfs.is_empty() {
            true => PATCH_FLAG_NONE.to_owned(),
            false => self
                .target_elfs
                .iter()
                .map(|(elf_name, _)| format!("{}, ", elf_name.to_string_lossy()))
                .collect::<String>()
                .trim_end_matches(", ")
                .to_string(),
        };

        log!(level, "uuid:        {}", self.uuid);
        log!(level, "name:        {}", self.name);
        log!(level, "version:     {}", self.version);
        log!(level, "release:     {}", self.release);
        log!(level, "arch:        {}", self.arch);
        log!(level, "type:        {}", self.kind);
        log!(level, "target:      {}", self.target.short_name());
        log!(level, "target_elf:  {}", target_elfs);
        log!(level, "digest:      {}", self.digest);
        log!(level, "license:     {}", self.license);
        log!(level, "description: {}", self.description);
        log!(level, "patch:");
        let mut patch_id = 1usize;
        for patch_file in &self.patches {
            log!(level, "{}. {}", patch_id, patch_file.name.to_string_lossy());
            patch_id += 1;
        }
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.uuid)
    }
}
