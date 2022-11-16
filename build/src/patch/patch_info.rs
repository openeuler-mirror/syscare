use std::collections::HashMap;
use std::path::Path;

use crate::statics::*;
use crate::util::fs;

use crate::cli::CliArguments;

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
pub struct PatchName {
    name:    String,
    version: String,
    release: String,
}

impl PatchName {
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

impl std::fmt::Display for PatchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}-{}", self.get_name(), self.get_version(), self.get_release()))
    }
}

impl std::str::FromStr for PatchName {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let str_slice = s.split(PKG_NAME_SPLITER).collect::<Vec<&str>>();
        if str_slice.len() != 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Parse patch name failed"),
            ));
        }

        Ok(Self {
            name:    str_slice[0].to_owned(),
            version: str_slice[1].to_owned(),
            release: str_slice[2].to_owned(),
        })
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

#[derive(Hash)]
#[derive(PartialEq, Eq)]
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
    pub fn new<P: AsRef<Path>>(file: P) -> std::io::Result<Self> {
        fs::check_file(&file)?;

        let file_path = file.as_ref().canonicalize()?;
        let name      = fs::stringtify_path(file_path.file_name().expect("Get patch name failed"));
        let path      = fs::stringtify_path(file_path.as_path().canonicalize()?);
        let digest    = fs::sha256_digest_file(file_path)?[..PATCH_VERSION_DIGITS].to_owned();

        Ok(Self { name, path, digest: digest })
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

    pub fn get_digest(&self) -> &str {
        &self.digest
    }

    pub fn set_digest(&mut self, value: String) {
        self.digest = value
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct PatchInfo {
    patch:      PatchName,
    patch_type: PatchType,
    target:     Option<PatchName>,
    license:    String,
    summary:    String,
    file_list:  Vec<PatchFile>,
}

impl PatchInfo {
    pub fn get_patch(&self) -> &PatchName {
        &self.patch
    }

    pub fn get_patch_type(&self) -> PatchType {
        self.patch_type
    }

    pub fn get_target(&self) -> Option<&PatchName> {
        self.target.as_ref()
    }

    pub fn get_file_list(&self) -> &[PatchFile] {
        self.file_list.as_slice()
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_summary(&self) -> &str {
        &self.summary
    }
}

impl PatchInfo {
    fn parse_patch(args: &CliArguments) -> std::io::Result<PatchName> {
        Ok(PatchName {
            name:    args.patch_name.to_owned(),
            version: args.patch_version.to_owned(),
            release: fs::sha256_digest_file_list(&args.patches)?[..PATCH_VERSION_DIGITS].to_string(),
        })
    }

    fn parse_patch_type(args: &CliArguments) -> PatchType {
        let find_result = fs::find_file(
            args.source.to_string(),
            KERNEL_SOURCE_DIR_FLAG,
            false,
            false,
        );

        match find_result.is_ok() {
            true  => PatchType::KernelPatch,
            false => PatchType::UserPatch,
        }
    }

    fn parse_target(args: &CliArguments) -> Option<PatchName> {
        match (args.target_name.clone(), args.target_version.clone(), args.target_release.clone()) {
            (Some(name), Some(version), Some(release)) => {
                Some(PatchName { name, version, release })
            },
            _ => None
        }
    }

    fn parse_license(args: &CliArguments) -> String {
        let license: Option<&str> = args.target_license.as_deref();
        license.unwrap_or(PATCH_UNDEFINED_VALUE).to_owned()
    }

    fn parse_summary(args: &CliArguments) -> String {
        args.patch_summary.to_owned()
    }

    fn parse_file_list(args: &CliArguments) -> std::io::Result<Vec<PatchFile>> {
        let mut file_map = HashMap::new();

        let mut patch_index = 1usize;
        for file in &args.patches {
            let mut patch_file = PatchFile::new(file)?;
            let patch_file_name = patch_file.get_name();
            // The patch file may come form patched source rpm, which is already renamed.
            if !patch_file_name.contains(PATCH_FILE_PREFIX) {
                // Patch file naming rule: ${prefix}-${patch_id}-${patch_name}
                let new_patch_name = format!("{}-{:04}-{}", PATCH_FILE_PREFIX, patch_index, patch_file_name);
                patch_file.set_name(new_patch_name);
            };

            let file_digest = patch_file.get_digest().to_owned();
            file_map.insert(file_digest, patch_file);
            patch_index += 1;
        }

        Ok(file_map.into_values().collect())
    }
}

impl std::fmt::Display for PatchInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let patch_target = self.get_target()
            .map(PatchName::to_string)
            .unwrap_or(PATCH_UNDEFINED_VALUE.to_string());

        f.write_fmt(format_args!("{}\n\n",        self.get_summary()))?;
        f.write_fmt(format_args!("name:    {}\n", self.get_patch().get_name()))?;
        f.write_fmt(format_args!("type:    {}\n", self.get_patch_type()))?;
        f.write_fmt(format_args!("version: {}\n", self.get_patch().get_version()))?;
        f.write_fmt(format_args!("release: {}\n", self.get_patch().get_release()))?;
        f.write_fmt(format_args!("license: {}\n", self.get_license()))?;
        f.write_fmt(format_args!("target:  {}\n", patch_target))?;
        f.write_str("\npatch list:")?;
        for patch_file in self.get_file_list() {
            f.write_fmt(format_args!("\n{} {}", patch_file.get_name(), patch_file.get_digest()))?;
        }

        Ok(())
    }
}

impl TryFrom<&CliArguments> for PatchInfo {
    type Error = std::io::Error;

    fn try_from(args: &CliArguments) -> Result<Self, Self::Error> {
        Ok(PatchInfo {
            patch:      Self::parse_patch(args)?,
            patch_type: Self::parse_patch_type(args),
            target:     Self::parse_target(args),
            license:    Self::parse_license(args),
            summary:    Self::parse_summary(args),
            file_list:  Self::parse_file_list(args)?
        })
    }
}
