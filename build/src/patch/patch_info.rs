use std::collections::{HashSet, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

use log::log;
use lazy_static::*;
use serde::{Serialize, Deserialize};

use crate::package::PackageInfo;
use crate::cli::CliArguments;

use crate::constants::*;

use crate::util::{fs, sys, digest};
use crate::util::os_str::OsStrContains;

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
    pub name:   String,
    pub path:   PathBuf,
    pub digest: String,
}

impl PatchFile {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }
        let mut file_digests = FILE_DIGESTS.lock().unwrap();

        let file_id = FILE_COUNTER.load(std::sync::atomic::Ordering::Relaxed);
        let file_path     = fs::realpath(path)?;
        let mut file_name = fs::file_name(&file_path)?;
        if !Self::validate_naming_rule(&file_name) {
            // The patch file may come form patched source rpm, which is already renamed.
            // Patch file naming rule: ${patch_id}-${patch_name}.patch
            file_name = format!("{:04}-{}", file_id, file_name);
        };

        let file_ext = fs::file_ext(&file_path)?;
        if file_ext != PATCH_FILE_EXTENSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is not a patch", file_name)
            ));
        }

        let file_digest = &digest::file_digest(file_path.as_path())?[..PATCH_VERSION_DIGITS];
        if !file_digests.insert(file_digest.to_owned()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is duplicated", file_name)
            ));
        }
        FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(Self {
            name:   file_name,
            path:   file_path,
            digest: file_digest.to_owned()
        })
    }

    pub fn validate_naming_rule(file_name: &str) -> bool {
        // Patch file naming rule: ${patch_id}-${patch_name}.patch
        let file_name_slice = file_name.split('-').collect::<Vec<_>>();
        if file_name_slice.len() < 2 {
            return false;
        }

        let patch_id = file_name_slice[0];
        if (patch_id.len() != 4) || patch_id.parse::<usize>().is_err() {
            return false;
        }

        let patch_name = file_name_slice.last().unwrap();
        if !patch_name.ends_with(PATCH_FILE_EXTENSION) {
            return false;
        }

        true
    }

    pub fn is_from_source_pkg(&self) -> bool {
        self.path.contains(sys::process_id().to_string())
    }
}

impl std::fmt::Display for PatchFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("PatchFile {{ name: {}, path: {}, digest: ${} }}",
            self.name,
            self.path.display(),
            self.digest
        ))
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PatchInfo {
    pub name:        String,
    pub version:     u32,
    pub release:     String,
    pub arch:        String,
    pub kind:        PatchType,
    pub target:      PackageInfo,
    pub target_elfs: HashMap<OsString, PathBuf>,
    pub license:     String,
    pub description: String,
    pub is_patched:  bool,
    pub builder:     String,
    pub patches:     Vec<PatchFile>,
}

impl PatchInfo {
    pub fn new(pkg_info: &PackageInfo, args: &CliArguments) -> std::io::Result<Self> {
        let name        = args.patch_name.to_owned();
        let kind        = match pkg_info.name == KERNEL_PKG_NAME {
            true  => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };
        let version     = args.patch_version;
        let release     = digest::file_list_digest(&args.patches)?[..PATCH_VERSION_DIGITS].to_string();
        let arch        = args.patch_arch.to_owned();
        let target      = pkg_info.to_owned();
        let target_elfs = HashMap::new();
        let license     = args.target_license.to_owned().unwrap();
        let description = args.patch_description.to_owned();
        let is_patched  = false;
        let builder     = CLI_VERSION.to_owned();
        let patches     = args.patches.iter().flat_map(|path| PatchFile::new(path)).collect();

        Ok(PatchInfo {
            name, kind,
            version, release, arch,
            target, target_elfs,
            license, description,
            is_patched, builder,
            patches
        })
    }
}

impl PatchInfo {
    fn get_target_elfs_str(&self) -> String {
        if self.target_elfs.is_empty() {
            return PATCH_FLAG_UNKNOWN.to_owned();
        }

        let mut str = String::new();
        for (elf_name, _) in self.target_elfs.iter() {
            str.push_str(&format!("{}, ", elf_name.to_string_lossy()));
        }
        str.pop();
        str.pop();
        str
    }

    pub fn print_log(&self, level: log::Level) {
        log!(level, "name:        {}", self.name);
        log!(level, "version:     {}", self.version);
        log!(level, "release:     {}", self.release);
        log!(level, "arch:        {}", self.arch);
        log!(level, "type:        {}", self.kind);
        log!(level, "target:      {}", self.target.short_name());
        log!(level, "target_elfs: {}", self.get_target_elfs_str());
        log!(level, "license:     {}", self.license);
        log!(level, "description: {}", self.description);
        log!(level, "builder:     {}", self.builder);
        log!(level, "");
        log!(level, "patch list:");
        for patch_file in &self.patches {
            log!(level, "{} {}", patch_file.name, patch_file.digest);
        }
    }
}
