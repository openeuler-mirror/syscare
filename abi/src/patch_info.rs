use std::{ffi::OsString, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::package_info::PackageInfo;

pub const PATCH_INFO_MAGIC: &str = "112574B6EDEE4BA4A05F";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
    pub fn name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const LIST_DISPLAY_LIMIT: usize = 9;

        writeln!(f, "------------------------------")?;
        writeln!(f, "Syscare Patch")?;
        writeln!(f, "------------------------------")?;
        if !self.uuid.is_empty() {
            writeln!(f, "uuid:        {}", self.uuid)?;
        }
        writeln!(f, "name:        {}", self.name)?;
        writeln!(f, "version:     {}", self.version)?;
        writeln!(f, "release:     {}", self.release)?;
        writeln!(f, "arch:        {}", self.arch)?;
        writeln!(f, "type:        {}", self.kind)?;
        writeln!(f, "target:      {}", self.target.short_name())?;
        writeln!(f, "license:     {}", self.target.license)?;
        writeln!(f, "description: {}", self.description)?;
        if !self.entities.is_empty() {
            writeln!(f, "entities:")?;
            for (entity_count, entity) in self.entities.iter().enumerate() {
                if entity_count >= LIST_DISPLAY_LIMIT {
                    writeln!(f, "* ......")?;
                    break;
                }
                writeln!(f, "* {}", entity.patch_name.to_string_lossy())?;
            }
        }
        if !self.patches.is_empty() {
            writeln!(f, "patches:")?;
            for (patch_count, patch_file) in self.patches.iter().enumerate() {
                if patch_count >= LIST_DISPLAY_LIMIT {
                    writeln!(f, "* ......")?;
                    break;
                }
                writeln!(f, "* {}", patch_file.name.to_string_lossy())?;
            }
        }
        write!(f, "------------------------------")?;

        Ok(())
    }
}
