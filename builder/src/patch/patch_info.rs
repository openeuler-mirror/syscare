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

const PATCH_DIGEST_LENGTH: usize = 8;
/*
 * In order to solve PatchInfo binary compatibility issue,
 * we use this version string to perform compatibility check
 * before PatchInfo deserialization.
 * Therefore, whenever the PatchInfo is modified (including PackageInfo),
 * it should be updated and keep sync with patch management cli.
 */
const PATCH_INFO_MAGIC: &str = "44C194B5C07832BD554531";

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
            .unwrap()
            .insert(digest.as_ref().to_owned())
    }

    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file_path = fs::canonicalize(path)?;
        let file_name = fs::file_name(&file_path);
        let file_digest = digest::file(&file_path)?[..PATCH_DIGEST_LENGTH].to_owned();

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
    pub digest: String,
    pub target: PackageInfo,
    pub entities: Vec<PatchEntity>,
    pub license: String,
    pub description: String,
    pub patches: Vec<PatchFile>,
}

impl PatchInfo {
    pub fn new(target_pkg_info: PackageInfo, args: &CliArguments) -> std::io::Result<Self> {
        const KERNEL_PKG_NAME: &str = "kernel";

        let uuid = Uuid::new_v4().to_string();
        let name = args.patch_name.to_owned();
        let kind = match target_pkg_info.name == KERNEL_PKG_NAME {
            true => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };
        let version = args.patch_version.to_owned();
        let release = args.patch_release;
        let arch = args.patch_arch.to_owned();
        let target = target_pkg_info;
        let entities = Vec::new();
        let digest = digest::file_list(&args.patches)?[..PATCH_DIGEST_LENGTH].to_owned();
        let license = args.target_license.to_owned().unwrap();
        let description = args.patch_description.to_owned();
        let patches = args.patches.iter().flat_map(PatchFile::new).collect();

        Ok(PatchInfo {
            uuid,
            name,
            kind,
            version,
            release,
            arch,
            target,
            entities,
            digest,
            license,
            description,
            patches,
        })
    }

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

        let patch_entities = match self.entities.is_empty() {
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
        log!(level, "target_elf:  {}", patch_entities);
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
