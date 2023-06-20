use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lazy_static::*;
use log::log;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::CliArguments;
use crate::package::PackageInfo;

use common::util::{digest, fs};

pub const PATCH_FILE_EXT: &str = "patch";
pub const PATCH_INFO_FILE_NAME: &str = "patch_info";
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

impl PatchEntity {
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(patch_file: P, elf_file: Q) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        Ok(Self {
            uuid: Uuid::new_v4().to_string(),
            patch_name: fs::file_name(patch_file.as_ref()),
            patch_target: elf_file.as_ref().to_owned(),
            checksum: digest::file(patch_file.as_ref())?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchFile {
    pub name: OsString,
    pub path: PathBuf,
    pub digest: String,
}

impl PatchFile {
    fn is_file_digest_exists<S: AsRef<str>>(digest: S) -> bool {
        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }
        FILE_DIGESTS
            .lock()
            .expect("Lock failed")
            .insert(digest.as_ref().to_owned())
    }

    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file_path = fs::canonicalize(path)?;
        let file_name = fs::file_name(&file_path);
        let file_digest = digest::file(&file_path)?;

        if !Self::is_file_digest_exists(&file_digest) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch \"{}\" is duplicated", file_path.display()),
            ));
        }

        Ok(Self {
            name: file_name,
            path: file_path,
            digest: file_digest,
        })
    }
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
                .map(|entity| {
                    format!(
                        "{}, ",
                        fs::file_name(&entity.patch_target).to_string_lossy()
                    )
                })
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

impl From<&CliArguments> for PatchInfo {
    fn from(args: &CliArguments) -> Self {
        const KERNEL_PKG_NAME: &str = "kernel";

        let target_package = PackageInfo::from(args);
        let patch_type = match target_package.name == KERNEL_PKG_NAME {
            true => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };

        Self {
            uuid: Uuid::new_v4().to_string(),
            name: args.patch_name.to_owned(),
            kind: patch_type,
            version: args.patch_version.to_owned(),
            release: args.patch_release.to_owned(),
            arch: args.patch_arch.to_owned(),
            target: target_package,
            entities: Vec::new(),
            description: args.patch_description.to_owned(),
            patches: args.patches.iter().flat_map(PatchFile::new).collect(),
        }
    }
}
