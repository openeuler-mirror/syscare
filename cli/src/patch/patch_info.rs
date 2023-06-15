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
pub const PATCH_INFO_MAGIC: &str = "112574B6EDEE4BA4A05F";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
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
pub struct PatchEntity {
    pub uuid: String,
    pub patch_name: OsString,
    pub patch_target: PathBuf,
    pub checksum: String,
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
    pub target: PackageInfo,
    pub entities: Vec<PatchEntity>,
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

    pub fn print_log(&self, level: log::Level) {
        const PATCH_FLAG_NONE: &str = "(none)";

        let patch_elfs = match self.entities.is_empty() {
            true => PATCH_FLAG_NONE.to_owned(),
            false => self
                .entities
                .iter()
                .map(|entity| format!("{}, ", entity.patch_name.to_string_lossy()))
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
        log!(level, "target_elf:  {}", patch_elfs);
        log!(level, "license:     {}", self.target.license);
        log!(level, "description: {}", self.description);
        log!(level, "patch:");
        let mut patch_id = 1usize;
        for patch_file in &self.patches {
            log!(level, "{}. {}", patch_id, patch_file.name.to_string_lossy());
            patch_id += 1;
        }
    }
}
