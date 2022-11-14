use std::path::Path;

use crate::cli::CliArguments;
use crate::util::fs;

const PATCH_FILE_HASH_DIGITS: usize = 8;
const PATCH_DEFAULT_VERSION:  &str  = "1";
const PATCH_DEFAULT_GROUP:    &str  = "Patch";
const PATCH_DEFAULT_SUMMARY:  &str  = "Syscare Patch";
const PATCH_UNDEFINED_VALUE:  &str  = "Undefined";

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
pub struct Version {
    name:    String,
    version: String,
    release: String,
}

impl Version {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}-{}", self.get_name(), self.get_version(), self.get_release()))
    }
}

#[derive(PartialEq)]
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
    name: String,
    path: String,
    hash: String,
}

impl PatchFile {
    pub fn new<P: AsRef<Path>>(file: P) -> std::io::Result<Self> {
        fs::check_file(&file)?;

        let file_path = file.as_ref().canonicalize()?;
        let name = fs::stringtify_path(file_path.file_name().expect("Get patch name failed"));
        let path = fs::stringtify_path(file_path.as_path());
        let hash = fs::sha256_digest_file(file_path)?[..PATCH_FILE_HASH_DIGITS].to_owned();

        Ok(Self { name, path, hash })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn set_path(&mut self, value: String) {
        self.path = value;
    }

    pub fn get_hash(&self) -> &str {
        &self.hash
    }

    pub fn set_hash(&mut self, value: String) {
        self.hash = value
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct PatchInfo {
    patch_version: Version,
    patch_type: PatchType,
    target_version: Option<Version>,
    license: String,
    summary: String,
    file_list: Vec<PatchFile>,
}

impl PatchInfo {
    pub fn get_patch_version(&self) -> &Version {
        &self.patch_version
    }

    pub fn get_patch_type(&self) -> PatchType {
        self.patch_type
    }

    pub fn get_target_version(&self) -> Option<&Version> {
        self.target_version.as_ref()
    }

    pub fn get_file_list(&self) -> &[PatchFile] {
        self.file_list.as_slice()
    }

    pub fn get_group(&self) -> &str {
        PATCH_DEFAULT_GROUP
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_summary(&self) -> &str {
        &self.summary
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let patch_target = self.get_target_version()
            .map(Version::to_string)
            .unwrap_or(PATCH_UNDEFINED_VALUE.to_string());

        f.write_fmt(format_args!("{}\n\n",        self.get_summary()))?;
        f.write_fmt(format_args!("name:    {}\n", self.get_patch_version().get_name()))?;
        f.write_fmt(format_args!("type:    {}\n", self.get_patch_type()))?;
        f.write_fmt(format_args!("version: {}\n", self.get_patch_version().get_version()))?;
        f.write_fmt(format_args!("release: {}\n", self.get_patch_version().get_release()))?;
        f.write_fmt(format_args!("license: {}\n", self.get_license()))?;
        f.write_fmt(format_args!("target:  {}\n", patch_target))?;
        f.write_str("\npatch list:")?;
        for patch_file in self.get_file_list() {
            f.write_fmt(format_args!("\n{} {}", patch_file.get_name(), patch_file.get_hash()))?;
        }

        Ok(())
    }
}

impl TryFrom<&CliArguments> for PatchInfo {
    type Error = std::io::Error;

    fn try_from(args: &CliArguments) -> Result<Self, Self::Error> {
        #[inline(always)]
        fn parse_patch_version(args: &CliArguments) -> std::io::Result<Version> {
            Ok(Version {
                name:    args.name.to_owned(),
                version: args.version.as_deref().unwrap_or(PATCH_DEFAULT_VERSION).to_owned(),
                release: fs::sha256_digest_file_list(&args.patches)?[..PATCH_FILE_HASH_DIGITS].to_string(),
            })
        }

        #[inline(always)]
        fn parse_patch_type(args: &CliArguments) -> PatchType {
            const KERNEL_FLAG_FILE_NAME: &str = "Kbuild";

            let find_result = fs::find_file(
                args.source.to_string(),
                KERNEL_FLAG_FILE_NAME,
                false,
                false,
            );

            match find_result.is_ok() {
                true  => PatchType::KernelPatch,
                false => PatchType::UserPatch,
            }
        }

        #[inline(always)]
        fn parse_target_version(args: &CliArguments) -> Option<Version> {
            match (args.target_name.clone(), args.target_version.clone(), args.target_release.clone()) {
                (Some(name), Some(version), Some(release)) => {
                    Some(Version { name, version, release })
                },
                _ => None
            }
        }

        #[inline(always)]
        fn parse_license(args: &CliArguments) -> String {
            let license: Option<&str> = args.target_license.as_deref();
            license.unwrap_or(PATCH_UNDEFINED_VALUE).to_owned()
        }

        #[inline(always)]
        fn parse_summary(args: &CliArguments) -> String {
            let summary: Option<&str> = args.summary.as_deref();
            summary.unwrap_or(PATCH_DEFAULT_SUMMARY).to_owned()
        }

        #[inline(always)]
        fn parse_file_list(args: &CliArguments) -> std::io::Result<Vec<PatchFile>> {
            let mut file_list = Vec::new();
            for file in &args.patches {
                file_list.push(PatchFile::new(file)?);
            }

            Ok(file_list)
        }

        Ok(PatchInfo {
            patch_version:  parse_patch_version(args)?,
            patch_type:     parse_patch_type(args),
            target_version: parse_target_version(args),
            license:        parse_license(args),
            summary:        parse_summary(args),
            file_list:      parse_file_list(args)?
        })
    }
}
