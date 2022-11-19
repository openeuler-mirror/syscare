use crate::constants::*;
use crate::util::fs;

use crate::workdir::PackageBuildRoot;
use crate::patch::{PatchType, PatchInfo};

pub struct RpmHelper;

impl RpmHelper {
    pub fn query_package_info(pkg_path: &str, format: &str) -> std::io::Result<String> {
        fs::check_file(pkg_path)?;

        let exit_status = RPM.execvp([ "--query", "--queryformat", format, pkg_path ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", RPM, exit_code),
            ));
        }

        Ok(exit_status.stdout().to_owned())
    }

    pub fn find_build_root(directory: &str) -> std::io::Result<PackageBuildRoot> {
        Ok(PackageBuildRoot::new(fs::stringtify(
            fs::find_directory(
                directory,
                PKG_BUILD_ROOT_DIR_NAME,
                false,
                true
            )?
        )))
    }

    pub fn find_spec_file(directory: &str) -> std::io::Result<String> {
        let spec_file = fs::find_file_ext(
            directory,
            PKG_SPEC_FILE_EXTENSION,
            false
        )?;

        Ok(fs::stringtify(spec_file))
    }

    pub fn find_source_directory(directory: &str, patch_info: &PatchInfo) -> std::io::Result<String> {
        let search_name = match patch_info.get_patch_type() {
            PatchType::UserPatch   => patch_info.get_patch().get_name(),
            PatchType::KernelPatch => KERNEL_SOURCE_DIR_PREFIX,
        };

        let source_dir = fs::find_directory(
            directory,
            search_name,
            true,
            true
        )?;

        Ok(fs::stringtify(source_dir))
    }
}
