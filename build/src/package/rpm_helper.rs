use std::path::{Path, PathBuf};

use log::debug;

use crate::constants::*;
use crate::util::fs;

use crate::workdir::PackageBuildRoot;
use crate::patch::{PatchType, PatchInfo};
use crate::cmd::ExternCommandArgs;

pub struct RpmHelper;

impl RpmHelper {
    pub fn query_package_info<P: AsRef<Path>>(pkg_path: P, format: &str) -> std::io::Result<String> {
        fs::check_file(&pkg_path)?;

        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("--query")
                .arg("--queryformat")
                .arg(format)
                .arg(pkg_path.as_ref().as_os_str())
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM, exit_code),
            ));
        }

        Ok(exit_status.stdout().to_owned())
    }

    pub fn find_build_root<P: AsRef<Path>>(directory: P) -> std::io::Result<PackageBuildRoot> {
        debug!("Finding package build root from '{}'", directory.as_ref().display());

        Ok(PackageBuildRoot::new(
            fs::find_directory(
                directory,
                PKG_BUILD_ROOT_DIR_NAME,
                false,
                true
            )?
        ))
    }

    pub fn find_spec_file<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding package spec file from '{}'", directory.as_ref().display());

        let spec_file = fs::find_file_ext(
            directory,
            PKG_SPEC_FILE_EXTENSION,
            false
        )?;

        Ok(spec_file)
    }

    pub fn find_source_directory<P: AsRef<Path>>(directory: P, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        debug!("Finding build source from '{}'", directory.as_ref().display());

        let search_name = match patch_info.get_type() {
            PatchType::UserPatch   => patch_info.get_target().get_name(),
            PatchType::KernelPatch => KERNEL_SOURCE_DIR_PREFIX,
        };

        let find_source_result = fs::find_directory(
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
                fs::find_directory(
                    &directory,
                    "",
                    true,
                    true
                )
            }
        }
    }
}
