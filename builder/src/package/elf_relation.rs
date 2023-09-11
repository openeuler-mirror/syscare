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
        let debuginfo_path = debuginfo.as_ref();

        let prefix = root.as_ref().join(DEBUGINFO_INSTALL_DIR);
        let suffix = format!(
            "-{}-{}.{}.{}",
            package.version, package.release, package.arch, DEBUGINFO_FILE_EXT
        );

        let elf_path = Path::new(
            debuginfo_path
                .as_os_str()
                .strip_prefix(prefix.as_os_str())
                .with_context(|| {
                    format!(
                        "Cannot parse elf path from \"{}\", prefix mismatched",
                        debuginfo_path.display()
                    )
                })?
                .strip_suffix(suffix)
                .with_context(|| {
                    format!(
                        "Cannot parse elf path from \"{}\", suffix mismatched",
                        debuginfo_path.display()
                    )
                })?,
        );

        Ok(ElfRelation {
            elf: elf_path.to_path_buf(),
            debuginfo: debuginfo_path.to_path_buf(),
        })
    }
}
