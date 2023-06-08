use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use common::util::ext_cmd::{ExternCommand, ExternCommandArgs};
use common::util::fs;
use common::util::os_str::OsStrExt;
use log::debug;

use crate::patch::{PatchInfo, PatchType};
use crate::workdir::PackageBuildRoot;

use super::package_info::PackageInfo;
use super::rpm_spec_helper::{RpmSpecHelper, SPEC_FILE_EXT};

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
    pub fn is_package_installed<S: AsRef<OsStr>>(pkg_name: S) -> bool {
        RPM.execvp(ExternCommandArgs::new().arg("--query").arg(&pkg_name))
            .map(|exit_status| exit_status.exit_code() == 0)
            .unwrap_or(false)
    }

    pub fn query_package_info<P: AsRef<Path>>(
        pkg_path: P,
        format: &str,
    ) -> std::io::Result<OsString> {
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
    ) -> std::io::Result<()> {
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
        .check_exit_code()
    }

    pub fn metadata_dir<P: AsRef<Path>>(pkg_source_dir: P) -> PathBuf {
        pkg_source_dir.as_ref().join(METADATA_DIR_NAME)
    }

    pub fn has_metadata<P: AsRef<Path>>(pkg_source_dir: P) -> bool {
        pkg_source_dir.as_ref().join(METADATA_PKG_NAME).exists()
    }

    pub fn compress_metadata<P: AsRef<Path>>(pkg_source_dir: P) -> std::io::Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_PKG_NAME);
        debug!("Compressing metadata into {:?}", metadata_file);

        TAR.execvp(
            ExternCommandArgs::new()
                .arg("-czf")
                .arg(metadata_file)
                .arg("-C")
                .arg(metadata_source_dir)
                .arg(METADATA_DIR_NAME)
                .arg("--restrict"),
        )?
        .check_exit_code()
    }

    pub fn decompress_medatadata<P: AsRef<Path>>(pkg_source_dir: P) -> std::io::Result<()> {
        let metadata_source_dir = pkg_source_dir.as_ref();
        let metadata_file = metadata_source_dir.join(METADATA_PKG_NAME);
        debug!("Decompressing metadata from {:?}", metadata_file);

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
        .check_exit_code()
    }

    pub fn add_metadata_to_spec_file<P: AsRef<Path>>(spec_file: P) -> std::io::Result<()> {
        RpmSpecHelper::add_files_to_spec(&spec_file, vec![METADATA_PKG_NAME])
    }

    pub fn find_build_root<P: AsRef<Path>>(directory: P) -> std::io::Result<PackageBuildRoot> {
        const PKG_BUILD_ROOT: &str = "rpmbuild";

        debug!("Finding package build root from {:?}", directory.as_ref());
        Ok(PackageBuildRoot::new(fs::find_dir(
            directory,
            PKG_BUILD_ROOT,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )?))
    }

    pub fn find_spec_file<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding package spec file from {:?}", directory.as_ref());
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
    ) -> std::io::Result<PathBuf> {
        const KERNEL_SOURCE_DIR_PREFIX: &str = "linux-";

        debug!("Finding package build source from {:?}", directory.as_ref());
        let search_name = match patch_info.kind {
            PatchType::UserPatch => &patch_info.target.name,
            PatchType::KernelPatch => KERNEL_SOURCE_DIR_PREFIX,
        };

        let find_source_result = fs::find_dir(
            &directory,
            search_name,
            fs::FindOptions {
                fuzz: true,
                recursive: true,
            },
        );

        match find_source_result {
            Ok(source_dir) => Ok(source_dir),
            Err(_) => fs::find_dir(
                &directory,
                "",
                fs::FindOptions {
                    fuzz: true,
                    recursive: true,
                },
            ),
        }
    }

    pub fn find_debuginfo<P: AsRef<Path>>(directory: P) -> std::io::Result<Vec<PathBuf>> {
        debug!("Finding package debuginfo from {:?}", directory.as_ref());

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
    ) -> std::io::Result<Vec<RpmElfRelation>>
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
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Cannot parse elf path from {:?}", debuginfo_path),
                    ));
                }
            }
        }

        Ok(elf_relations)
    }
}
