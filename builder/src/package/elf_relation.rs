use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use syscare_abi::PackageInfo;
use syscare_common::util::os_str::OsStrExt;

use super::{DEBUGINFO_FILE_EXT, DEBUGINFO_INSTALL_DIR};

#[derive(Debug, Clone)]
pub struct ElfRelation {
    pub elf: PathBuf,
    pub debuginfo: PathBuf,
}

impl ElfRelation {
    pub fn parse_from<P, Q>(root: P, package: &PackageInfo, debuginfo: Q) -> Result<ElfRelation>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let prefix = root.as_ref().join(DEBUGINFO_INSTALL_DIR);

        let debuginfo_path = debuginfo.as_ref().to_path_buf();
        let elf_path = debuginfo_path
            .as_os_str()
            .strip_prefix(prefix.as_os_str())
            .and_then(|name| {
                // %{name}-%{version}-%{release}-%{arch}.debug
                if let Some(s) = name.strip_suffix(format!(
                    "-{}-{}.{}.{}",
                    package.version, package.release, package.arch, DEBUGINFO_FILE_EXT
                )) {
                    return Some(s);
                }
                // %{name}.debug
                if let Some(s) = name.strip_suffix(format!(".{}", DEBUGINFO_FILE_EXT)) {
                    return Some(s);
                }
                None
            })
            .map(PathBuf::from)
            .with_context(|| {
                format!(
                    "Cannot parse elf path from \"{}\", suffix mismatched",
                    debuginfo_path.display()
                )
            })?;

        Ok(ElfRelation {
            elf: elf_path,
            debuginfo: debuginfo_path,
        })
    }
}
