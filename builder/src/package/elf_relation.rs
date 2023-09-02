use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use syscare_abi::PackageInfo;
use syscare_common::util::os_str::OsStrExt;

use super::{DEBUGINFO_FILE_EXT, DEBUGINFO_INSTALL_DIR};

pub struct ElfRelation {
    pub elf: PathBuf,
    pub debuginfo: PathBuf,
}

impl ElfRelation {
    pub fn parse_from<I, P, Q>(
        debuginfos: I,
        root: P,
        target_pkg: &PackageInfo,
    ) -> Result<Vec<ElfRelation>>
    where
        I: IntoIterator<Item = Q>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut elf_relations = Vec::new();
        for debuginfo in debuginfos {
            let debuginfo_path = debuginfo.as_ref();

            let prefix = root.as_ref().join(DEBUGINFO_INSTALL_DIR);
            let suffix = format!(
                "-{}-{}.{}.{}",
                target_pkg.version, target_pkg.release, target_pkg.arch, DEBUGINFO_FILE_EXT
            );

            let elf_path = debuginfo_path
                .as_os_str()
                .strip_suffix(suffix)
                .and_then(|path| path.strip_prefix(prefix.as_os_str()))
                .map(PathBuf::from);

            match elf_path {
                Some(elf) => {
                    elf_relations.push(ElfRelation {
                        elf,
                        debuginfo: debuginfo_path.to_path_buf(),
                    });
                }
                None => {
                    bail!(
                        "Cannot parse elf path from \"{}\"",
                        debuginfo_path.display()
                    )
                }
            }
        }

        Ok(elf_relations)
    }
}
