use crate::statics::*;
use crate::patch::{PatchFile, Version};
use crate::util::fs;

use super::rpm_buildroot::RpmBuildRoot;

pub struct RpmBuilder {
    build_root: RpmBuildRoot
}

impl RpmBuilder {
    pub fn new(build_root: &str) -> Self {
        Self { build_root: RpmBuildRoot::new(&build_root) }
    }

    pub fn copy_patch_file_to_source(&self, patch_list: &[PatchFile]) -> std::io::Result<()> {
        for patch_file in patch_list {
            let src_path = patch_file.get_path();
            fs::check_file(src_path)?;

            let dst_path = format!("{}/{}", self.build_root.get_source_path(), patch_file.get_name());
            std::fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }

    pub fn build_source_package(&self, spec_file_path: &str, patch_version: &Version, output_dir: &str) -> std::io::Result<()> {
        fs::check_file(spec_file_path)?;
        fs::check_dir(output_dir)?;

        let patch_release = format!(".{}.{}.{}.{}", PKG_FLAG_PATCHED_SOURCE_PKG,
            patch_version.get_name(), patch_version.get_version(), patch_version.get_release()
        );

        // Build source rpm
        let exit_status = RPM_BUILD.execvp([
            "--define", &format!("_topdir {}", self.build_root),
            "--define", &format!("{} {}", PKG_SPEC_MACRO_PATCH_RELEASE_NAME, patch_release),
            "-bs", spec_file_path
        ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        // Copy source rpm to output directory
        fs::copy_all_files(self.build_root.get_srpm_path(), output_dir)?;

        Ok(())
    }

    pub fn build_binary_package(&self, spec_file_path: &str, output_dir: &str) -> std::io::Result<()> {
        fs::check_file(spec_file_path)?;
        fs::check_dir(output_dir)?;

        let exit_status = RPM_BUILD.execvp([
            "--define", &format!("_topdir {}", self.build_root),
            "-bb", spec_file_path
        ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", RPM_BUILD, exit_code),
            ));
        }

        fs::copy_all_files(self.build_root.get_rpm_path(), output_dir)?;

        Ok(())
    }
}

impl From<RpmBuildRoot> for RpmBuilder {
    fn from(build_root: RpmBuildRoot) -> Self {
        Self { build_root }
    }
}
