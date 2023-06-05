use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};
use std::sync::Mutex;

use log::log;
use lazy_static::*;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::package::PackageInfo;
use crate::cli::CliArguments;

use common::util::{fs, digest};

const PATCH_DIGEST_LENGTH: usize = 8;
/*
 * In order to solve PatchInfo binary compatibility issue,
 * we use this version string to perform compatibility check
 * before PatchInfo deserialization.
 * Therefore, whenever the PatchInfo is modified (including PackageInfo),
 * it should be updated and keep sync with patch management cli.
 */
const PATCH_INFO_MAGIC: &str = "2B96A33EC26809077";

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

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PatchFile {
    pub name:   OsString,
    pub path:   PathBuf,
    pub digest: String,
}

impl PatchFile {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        let file_path = fs::canonicalize(path)?;
        let file_name = fs::file_name(&file_path);

        let mut file_digests = FILE_DIGESTS.lock().unwrap();
        let file_digest = &digest::file(file_path.as_path())?[..PATCH_DIGEST_LENGTH];
        if !file_digests.insert(file_digest.to_owned()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Patch \"{}\" is duplicated", file_path.display())
            ));
        }

        Ok(Self {
            name:   file_name.to_owned(),
            path:   file_path,
            digest: file_digest.to_owned()
        })
    }
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PatchInfo {
    pub uuid:        String,
    pub name:        String,
    pub version:     String,
    pub release:     u32,
    pub arch:        String,
    pub kind:        PatchType,
    pub digest:      String,
    pub target:      PackageInfo,
    pub target_elfs: HashMap<OsString, PathBuf>, // (elf_name, elf_path)
    pub license:     String,
    pub description: String,
    pub patches:     Vec<PatchFile>,
}

impl PatchInfo {
    pub fn new(target_pkg_info: PackageInfo, args: &CliArguments) -> std::io::Result<Self> {
        const KERNEL_PKG_NAME: &str = "kernel";

        let uuid        = Uuid::new_v4().to_string();
        let name        = args.patch_name.to_owned();
        let kind        = match target_pkg_info.name == KERNEL_PKG_NAME {
            true  => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };
        let version     = args.patch_version.to_owned();
        let release     = args.patch_release;
        let arch        = args.patch_arch.to_owned();
        let target      = target_pkg_info;
        let target_elfs = HashMap::new();
        let digest      = digest::file_list(&args.patches)?[..PATCH_DIGEST_LENGTH].to_owned();
        let license     = args.target_license.to_owned().unwrap();
        let description = args.patch_description.to_owned();
        let patches     = args.patches.iter().flat_map(|path| PatchFile::new(path)).collect();

        Ok(PatchInfo {
            uuid, name, kind,
            version, release, arch,
            target, target_elfs, digest,
            license, description,
            patches,
        })
    }

    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!("{}-{}-{}.{}", self.name, self.version, self.release, self.arch)
    }

    pub fn version() -> &'static str {
        PATCH_INFO_MAGIC
    }
}

impl PatchInfo {
    pub fn print_log(&self, level: log::Level) {
        const PATCH_FLAG_NONE: &str = "(none)";

        let target_elfs = match self.target_elfs.is_empty() {
            true => {
                PATCH_FLAG_NONE.to_owned()
            },
            false => {
                self.target_elfs.iter().map(|(elf_name, _)| {
                    format!("{}, ", elf_name.to_string_lossy())
                }).collect::<String>().trim_end_matches(", ").to_string()
            },
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
        for patch_file in &self.patches {
            log!(level, "{} {}", patch_file.digest, patch_file.name.to_string_lossy());
        }
    }
}
