use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lazy_static::*;
use serde::{Serialize, Deserialize};

use crate::package::PackageInfo;
use crate::cli::CliArguments;

use crate::constants::*;

use crate::util::{fs, sys, sha256};
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
    name:   String,
    path:   PathBuf,
    digest: String,
}

impl PatchFile {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        lazy_static! {
            static ref FILE_COUNTER: Mutex<usize>           = Mutex::new(1);
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        let mut file_id      = FILE_COUNTER.lock().unwrap();
        let mut file_digests = FILE_DIGESTS.lock().unwrap();

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

        let file_digest = &sha256::file_digest(file_path.as_path())?[..PATCH_VERSION_DIGITS];
        if !file_digests.insert(file_digest.to_owned()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is duplicated", file_name)
            ));
        }
        *file_id += 1;

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
        self.path.contains(sys::get_process_id().to_string())
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_digest(&self) -> &str {
        &self.digest
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
    pub fn new(pkg_info: &PackageInfo, args: &CliArguments) -> std::io::Result<Self> {
        let name        = args.patch_name.to_owned();
        let kind        = match pkg_info.get_name() == KERNEL_PKG_NAME {
            true  => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        };
        let arch        = args.patch_arch.to_owned();
        let version     = args.patch_version;
        let release     = sha256::file_list_digest(&args.patches)?[..PATCH_VERSION_DIGITS].to_string();
        let target      = pkg_info.to_owned();
        let elf_name    = args.target_elfname.to_owned().unwrap();
        let license     = args.target_license.to_owned().unwrap();
        let description = args.patch_description.to_owned();
        let incremental = false;
        let builder     = format!("{} {}", CLI_NAME, CLI_VERSION);
        let patches     = args.patches
            .iter()
            .flat_map(|path| PatchFile::new(path))
            .collect();

        Ok(PatchInfo {
            name, kind, arch,
            version, release,
            target,  elf_name,
            license, description,
            incremental, builder,
            patches
        })
    }

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

    pub fn is_incremental(&self) -> bool {
        self.incremental
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
            format!("Deserialize path info failed, {}", e)
        ))
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        std::fs::write(
            path,
            bincode::serialize(self).map_err(|e| std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Serialize path info failed, {}", e)
            ))?
        )
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
            f.write_fmt(format_args!("\n{} {}", patch_file.get_name(), patch_file.get_digest()))?;
        }

        Ok(())
    }
}
