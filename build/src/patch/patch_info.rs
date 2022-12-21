use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use lazy_static::*;

use crate::constants::*;
use crate::util::fs;

use crate::package::PackageInfo;
use crate::cli::CliArguments;

#[derive(Clone, Copy)]
#[derive(Debug)]
pub enum PatchType {
    UserPatch,
    KernelPatch,
}

impl std::fmt::Display for PatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct PatchFile {
    name:   String,
    path:   String,
    digest: String,
}

impl std::fmt::Display for PatchFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("PatchFile {{ name: {}, path: {}, digest: ${} }}",
            self.name,
            self.path,
            self.digest
        ))
    }
}

impl PatchFile {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Option<Self>> {
        lazy_static! {
            static ref FILE_COUNTER: Mutex<usize>           = Mutex::new(1);
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        let mut file_id      = FILE_COUNTER.lock().unwrap();
        let mut file_digests = FILE_DIGESTS.lock().unwrap();

        let file_path     = fs::realpath(path)?;
        let mut file_name = fs::file_name(file_path.as_path())?;
        if !Self::validate_naming_rule(&file_name) {
            // The patch file may come form patched source rpm, which is already renamed.
            // Patch file naming rule: ${patch_id}-${patch_name}.patch
            file_name = format!("{:04}-{}", file_id, file_name);
        };

        let file_ext = fs::file_ext(file_path.as_path())?;
        if file_ext != PATCH_FILE_EXTENSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is not a patch", file_name)
            ));
        }

        let file_digest = &fs::sha256_digest_file(file_path.as_path())?[..PATCH_VERSION_DIGITS];
        if !file_digests.insert(file_digest.to_owned()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("File {} is duplicated", file_name)
            ));
        }

        *file_id += 1;

        Ok(Some(Self {
            name:   file_name,
            path:   fs::stringtify(file_path.as_path()),
            digest: file_digest.to_owned()
        }))
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

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_digest(&self) -> &str {
        &self.digest
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct PatchInfo {
    name:            String,
    kind:            PatchType,
    version:         String,
    release:         String,
    target:          PackageInfo,
    target_elf_name: String,
    description:     String,
    builder:         String,
    file_list:       Vec<PatchFile>,
}

impl PatchInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> PatchType {
        self.kind
    }

    pub fn get_arch(&self) -> &str {
        self.target.get_arch()
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn get_target(&self) -> &PackageInfo {
        &self.target
    }

    pub fn get_target_elf_name(&self) -> &str {
        &self.target_elf_name
    }

    pub fn get_license(&self) -> &str {
        &self.target.get_license()
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_builder(&self) -> &str {
        &self.builder
    }

    pub fn get_file_list(&self) -> &[PatchFile] {
        self.file_list.as_slice()
    }
}

impl PatchInfo {
    fn parse_patch_name(args: &CliArguments) -> String {
        args.patch_name.to_owned()
    }

    fn parse_patch_version(args: &CliArguments) -> String {
        args.patch_version.to_owned()
    }

    fn parse_patch_release(args: &CliArguments) -> std::io::Result<String> {
        Ok(fs::sha256_digest_file_list(&args.patches)?[..PATCH_VERSION_DIGITS].to_string())
    }

    fn parse_patch_type(pkg_info: &PackageInfo) -> PatchType {
        match pkg_info.get_name() == KERNEL_PKG_NAME {
            true  => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        }
    }

    fn parse_target(args: &CliArguments) -> std::io::Result<PackageInfo> {
        let query_str = format!("{}|{}|{}|{}|{}|{}|{}",
            args.target_name.as_ref().expect("Parse target name failed").to_owned(),
            args.target_arch.as_ref().expect("Parse target arch failed").to_owned(),
            args.target_epoch.as_ref().expect("Parse target epoch failed").to_owned(),
            args.target_version.as_ref().expect("Parse target version failed").to_owned(),
            args.target_release.as_ref().expect("Parse target release failed").to_owned(),
            args.target_license.as_ref().expect("Parse target license failed").to_owned(),
            PKG_FLAG_NONE
        );
        query_str.parse::<PackageInfo>()
    }

    fn parse_target_elf_name(args: &CliArguments) -> String {
        args.target_elfname.as_ref()
            .expect("Parse target elf name failed")
            .to_owned()
    }

    fn parse_description(args: &CliArguments) -> String {
        args.patch_description.to_owned()
    }

    fn parse_builder() -> String {
        format!("{} {}", CLI_NAME, CLI_VERSION)
    }

    fn parse_file_list(args: &CliArguments) -> std::io::Result<Vec<PatchFile>> {
        let mut patch_file_list = Vec::new();

        for file_path in &args.patches {
            if let Some(patch_file) = PatchFile::new(file_path)? {
                patch_file_list.push(patch_file);
            }
        }

        Ok(patch_file_list)
    }

    pub fn parse_from(pkg_info: &PackageInfo, args: &CliArguments) -> std::io::Result<Self> {
        Ok(PatchInfo {
            name:            Self::parse_patch_name(args),
            version:         Self::parse_patch_version(args),
            release:         Self::parse_patch_release(args)?,
            kind:            Self::parse_patch_type(pkg_info),
            target:          Self::parse_target(args)?,
            target_elf_name: Self::parse_target_elf_name(args),
            description:     Self::parse_description(args),
            builder:         Self::parse_builder(),
            file_list:       Self::parse_file_list(args)?
        })
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:        {}\n", self.get_name()))?;
        f.write_fmt(format_args!("type:        {}\n", self.get_type()))?;
        f.write_fmt(format_args!("arch:        {}\n", self.get_arch()))?;
        f.write_fmt(format_args!("target:      {}\n", self.get_target().get_simple_name()))?;
        f.write_fmt(format_args!("elf_name:    {}\n", self.get_target_elf_name()))?;
        f.write_fmt(format_args!("license:     {}\n", self.get_license()))?;
        f.write_fmt(format_args!("version:     {}\n", self.get_version()))?;
        f.write_fmt(format_args!("release:     {}\n", self.get_release()))?;
        f.write_fmt(format_args!("description: {}\n", self.get_description()))?;
        f.write_fmt(format_args!("builder:     {}\n", self.get_builder()))?;
        f.write_str("\npatch list:")?;
        for patch_file in self.get_file_list() {
            f.write_fmt(format_args!("\n{} {}", patch_file.get_name(), patch_file.get_digest()))?;
        }

        Ok(())
    }
}
