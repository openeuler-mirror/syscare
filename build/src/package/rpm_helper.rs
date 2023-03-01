use std::ffi::OsString;
use std::path::{Path, PathBuf};

use log::debug;

use crate::workdir::PackageBuildRoot;
use crate::patch::{PatchType, PatchInfo};

use crate::util::fs;
use crate::util::ext_cmd::{ExternCommand, ExternCommandArgs};

use super::rpm_spec_helper::SPEC_FILE_EXT;

pub const PKG_FILE_EXT: &str = "rpm";

pub(super) const RPM:       ExternCommand = ExternCommand::new("rpm");
pub(super) const RPM_BUILD: ExternCommand = ExternCommand::new("rpmbuild");

pub struct RpmHelper;

impl RpmHelper {
    pub fn query_package_info<P: AsRef<Path>>(pkg_path: P, format: &str) -> std::io::Result<OsString> {
        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("--query")
                .arg("--queryformat")
                .arg(format)
                .arg(pkg_path.as_ref().as_os_str())
        )?;
        exit_status.check_exit_code()?;

        Ok(exit_status.stdout().to_owned())
    }

    pub fn find_build_root<P: AsRef<Path>>(directory: P) -> std::io::Result<PackageBuildRoot> {
        const PKG_BUILD_ROOT: &str = "rpmbuild";

        debug!("Finding package build root from \"{}\"", directory.as_ref().display());
        Ok(PackageBuildRoot::new(
            fs::find_dir(
                directory,
                PKG_BUILD_ROOT,
                false,
                true
            )?
        ))
    }

    pub fn find_spec_file<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding package spec file from \"{}\"", directory.as_ref().display());

        let spec_file = fs::find_file_ext(
            directory,
            SPEC_FILE_EXT,
            false
        )?;

        Ok(spec_file)
    }

    pub fn find_source_directory<P: AsRef<Path>>(directory: P, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        const KERNEL_SOURCE_DIR_PREFIX: &str = "linux-";

        debug!("Finding build source from \"{}\"", directory.as_ref().display());
        let search_name = match patch_info.kind {
            PatchType::UserPatch   => &patch_info.target.name,
            PatchType::KernelPatch => KERNEL_SOURCE_DIR_PREFIX,
        };

        let find_source_result = fs::find_dir(
            &directory,
            search_name,
            true,
            true
        );

        match find_source_result {
            Ok(source_dir) => {
                Ok(source_dir)
            },
            Err(_) => {
                fs::find_dir(
                    &directory,
                    "",
                    true,
                    true
                )
            }
        }
    }
}
