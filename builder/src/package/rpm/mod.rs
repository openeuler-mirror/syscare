use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use lazy_static::lazy_static;
use syscare_abi::{PackageInfo, PackageType};
use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    fs,
    os_str::OsStrExt,
};

mod pkg_builder;
mod spec_builder;
mod spec_file;
mod spec_writer;
mod tags;

pub use pkg_builder::RpmPackageBuilder;
pub use spec_builder::RpmSpecBuilder;
pub use spec_writer::RpmSpecWriter;

use super::{ElfRelation, Package, PackageBuildRoot, DEBUGINFO_FILE_EXT};

lazy_static! {
    pub static ref RPM: ExternCommand = ExternCommand::new("rpm");
}
pub const PKG_FILE_EXT: &str = "rpm";
pub const SPEC_FILE_EXT: &str = "spec";
pub const SPEC_TAG_VALUE_NONE: &str = "(none)";
pub const SPEC_SCRIPT_VALUE_NONE: &str = "# None";

const PKG_BUILD_ROOT: &str = "rpmbuild";

pub struct RpmPackage;

impl RpmPackage {
    fn query_package_info<P: AsRef<Path>>(pkg_path: P, format: &str) -> Result<OsString> {
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
}

impl Package for RpmPackage {
    fn extension(&self) -> &'static str {
        PKG_FILE_EXT
    }

    fn parse_package_info(&self, pkg_path: &Path) -> Result<PackageInfo> {
        let query_result = Self::query_package_info(
            pkg_path,
            "%{NAME}|%{ARCH}|%{EPOCH}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}",
        )?
        .to_string_lossy()
        .to_string();

        let pkg_info = query_result.split('|').collect::<Vec<_>>();
        if pkg_info.len() < 7 {
            bail!("Parse package info from \"{}\" failed", pkg_path.display());
        }

        let name = pkg_info[0].to_owned();
        let kind = match pkg_info[6] == SPEC_TAG_VALUE_NONE {
            true => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };
        let arch = pkg_info[1].to_owned();
        let epoch = pkg_info[2].to_owned();
        let version = pkg_info[3].to_owned();
        let release = pkg_info[4].to_owned();
        let license = pkg_info[5].to_owned();
        let source_pkg = match kind {
            // For source package, it doesn't have %SOURCERPM, we reuse this field to store file name
            PackageType::SourcePackage => fs::file_name(pkg_path).to_string_lossy().to_string(),
            PackageType::BinaryPackage => pkg_info[6].to_owned(),
        };

        Ok(PackageInfo {
            name,
            kind,
            arch,
            epoch,
            version,
            release,
            license,
            source_pkg,
        })
    }

    fn query_package_files(&self, pkg_path: &Path) -> Result<Vec<PathBuf>> {
        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("--query")
                .arg("--list")
                .arg("--package")
                .arg(pkg_path),
        )?;
        exit_status.check_exit_code()?;

        let mut file_list = Vec::new();
        for file in exit_status.stdout().split('\n') {
            file_list.push(PathBuf::from(file));
        }

        Ok(file_list)
    }

    fn parse_elf_relations(
        &self,
        package: &PackageInfo,
        debuginfo_root: &Path,
    ) -> Result<Vec<ElfRelation>> {
        let debuginfo_files = fs::list_files_by_ext(
            debuginfo_root,
            DEBUGINFO_FILE_EXT,
            fs::TraverseOptions { recursive: true },
        )?;
        if debuginfo_files.is_empty() {
            bail!("Cannot find any debuginfo file");
        }

        let mut elf_relations = Vec::new();
        for debuginfo in &debuginfo_files {
            // Skip elf relation error check may cause unknown error
            if let Ok(elf_relation) = ElfRelation::parse_from(debuginfo_root, package, debuginfo) {
                elf_relations.push(elf_relation);
            }
        }
        Ok(elf_relations)
    }

    fn extract_package(&self, pkg_path: &Path, output_dir: &Path) -> Result<()> {
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
                .arg(output_dir)
                .arg("--package")
                .arg(pkg_path),
        )?
        .check_exit_code()?;

        Ok(())
    }

    fn find_build_root(&self, directory: &Path) -> Result<PackageBuildRoot> {
        let build_root = fs::find_dir(
            directory,
            PKG_BUILD_ROOT,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )?;
        PackageBuildRoot::new(build_root)
    }

    fn find_spec_file(&self, directory: &Path, package_name: &str) -> Result<PathBuf> {
        let file_name = format!("{}.{}", package_name, SPEC_FILE_EXT);
        let spec_file = fs::find_file(
            directory,
            file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;
        Ok(spec_file)
    }

    fn find_source_directory(&self, directory: &Path, package_name: &str) -> Result<PathBuf> {
        let build_source = fs::find_dir(
            directory,
            package_name,
            fs::FindOptions {
                fuzz: true,
                recursive: true,
            },
        )
        .or_else(|_| {
            fs::find_dir(
                directory,
                "",
                fs::FindOptions {
                    fuzz: true,
                    recursive: true,
                },
            )
        })?;

        Ok(build_source)
    }
}
