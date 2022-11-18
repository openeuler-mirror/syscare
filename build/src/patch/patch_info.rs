use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use lazy_static::*;

use crate::constants::*;
use crate::util::fs;

use crate::cli::CliArguments;

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
            eprintln!("File {} is not a patch", file_name);
            return Ok(None);
        }

        let file_digest = &fs::sha256_digest_file(file_path.as_path())?[..PATCH_VERSION_DIGITS];
        if !file_digests.insert(file_digest.to_owned()) {
            eprintln!("Patch file '{}' is duplicated", file_name);
            return Ok(None);
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
        let file_name_slice = file_name.split(PATCH_NAME_SPLITER).collect::<Vec<_>>();
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
        let mut patch_file_list = Vec::new();

        for file_path in &args.patches {
            if let Some(patch_file) = PatchFile::new(file_path)? {
                patch_file_list.push(patch_file);
            }
        }

        Ok(patch_file_list)
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
