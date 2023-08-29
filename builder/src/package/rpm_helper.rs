use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use syscare_abi::{PackageInfo, PatchInfo, PatchType};
use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    fs,
    os_str::OsStrExt,
};

use crate::workdir::RpmBuildRoot;

use super::{rpm_spec_helper::SPEC_FILE_EXT, RpmSpecHelper};

pub const PKG_FILE_EXT: &str = "rpm";
pub const DEBUGINFO_FILE_EXT: &str = "debug";

pub(super) const TAR: ExternCommand = ExternCommand::new("tar");
pub(super) const RPM: ExternCommand = ExternCommand::new("rpm");
pub(super) const RPM_BUILD: ExternCommand = ExternCommand::new("rpmbuild");

const METADATA_PKG_NAME: &str = ".syscare.tar.gz";
const METADATA_DIR_NAME: &str = ".syscare";

pub struct RpmElfRelation {
    pub elf: PathBuf,
    pub debuginfo: PathBuf,
}

pub struct RpmHelper;

impl RpmHelper {
    pub fn query_package_info<P: AsRef<Path>>(pkg_path: P, format: &str) -> Result<OsString> {
        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("--query")
                .arg("--queryformat")
                .arg(format)
                .arg("--package")
                .arg(pkg_path.as_ref().as_os_str()),
        )?;
        exit_status.check_exit_code()?;

        Ok(exit_status.stdout().to_owned())
    }

    pub fn extract_package<P: AsRef<Path>, Q: AsRef<Path>>(
        pkg_path: P,
        output_dir: Q,
    ) -> Result<()> {
        RPM.execvp(
            ExternCommandArgs::new()
                .arg("--install")
                .arg("--nodeps")
                .arg("--nofiledigest")
                .arg("--nocontexts")
                .arg("--nocaps")
                .arg("--noscripts")
                .arg("--notriggers")
                .arg("--nodigest")
                .arg("--nofiledigest")
                .arg("--allfiles")
                .arg("--root")
                .arg(output_dir.as_ref())
                .arg("--package")
                .arg(pkg_path.as_ref()),
        )?
        .check_exit_code()?;

        Ok(())
    }

    pub fn metadata_dir<P: AsRef<Path>>(pkg_source_dir: P) -> PathBuf {
        pkg_source_dir.as_ref().join(METADATA_DIR_NAME)
    }

    pub fn has_metadata<P: AsRef<Path>>(pkg_source_dir: P) -> bool {
        pkg_source_dir.as_ref().join(METADATA_PKG_NAME).exists()
    }

    pub fn compress_metadata<P: AsRef<Path>>(pkg_source_dir: P) -> Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_PKG_NAME);

        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-czf")
                .arg(metadata_file)
                .arg("-C")
                .arg(metadata_source_dir)
                .arg(METADATA_DIR_NAME)
                .arg("--restrict"),
        )?
        .check_exit_code()?;

        Ok(())
    }

    pub fn decompress_medatadata<P: AsRef<Path>>(pkg_source_dir: P) -> Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_PKG_NAME);

        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-xf")
                .arg(metadata_file)
                .arg("-C")
                .arg(metadata_source_dir)
                .arg("--no-same-owner")
                .arg("--no-same-permissions")
                .arg("--restrict"),
        )?
        .check_exit_code()?;

        Ok(())
    }

    pub fn add_metadata_to_spec<P: AsRef<Path>>(spec_file: P) -> Result<()> {
        Ok(RpmSpecHelper::add_files_to_spec(
            &spec_file,
            vec![METADATA_PKG_NAME],
        )?)
    }

    pub fn find_rpmbuild_root<P: AsRef<Path>>(directory: P) -> Result<RpmBuildRoot> {
        const PKG_BUILD_ROOT: &str = "rpmbuild";

        RpmBuildRoot::new(fs::find_dir(
            directory,
            PKG_BUILD_ROOT,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )?)
    }

    pub fn find_spec_file<P: AsRef<Path>>(directory: P) -> Result<PathBuf> {
        let spec_file = fs::find_file_by_ext(
            directory,
            SPEC_FILE_EXT,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;

        Ok(spec_file)
    }

    pub fn find_build_source<P: AsRef<Path>>(
        directory: P,
        patch_info: &PatchInfo,
    ) -> Result<PathBuf> {
        const KERNEL_SOURCE_DIR_PREFIX: &str = "linux-";

        let search_name = match patch_info.kind {
            PatchType::UserPatch => &patch_info.target.name,
            PatchType::KernelPatch => KERNEL_SOURCE_DIR_PREFIX,
        };

        let build_source = fs::find_dir(
            &directory,
            search_name,
            fs::FindOptions {
                fuzz: true,
                recursive: true,
            },
        )
        .or_else(|_| {
            fs::find_dir(
                &directory,
                "",
                fs::FindOptions {
                    fuzz: true,
                    recursive: true,
                },
            )
        })?;

        Ok(build_source)
    }

    pub fn find_debuginfo<P: AsRef<Path>>(directory: P) -> Result<Vec<PathBuf>> {
        let debuginfo_files = fs::list_files_by_ext(
            &directory,
            DEBUGINFO_FILE_EXT,
            fs::TraverseOptions { recursive: true },
        )?;

        Ok(debuginfo_files)
    }

    pub fn parse_elf_relations<I, P, Q>(
        debuginfos: I,
        root: P,
        target_pkg: &PackageInfo,
    ) -> Result<Vec<RpmElfRelation>>
    where
        I: IntoIterator<Item = Q>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        const DEBUGINFO_INSTALL_DIR: &str = "usr/lib/debug";

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
                    elf_relations.push(RpmElfRelation {
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
