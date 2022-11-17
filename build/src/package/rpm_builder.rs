use crate::patch::PatchInfo;

use crate::constants::*;
use crate::util::fs;

use super::rpm_spec_generator::RpmSpecGenerator;
use super::rpm_buildroot::RpmBuildRoot;

pub struct RpmBuilder {
    build_root: RpmBuildRoot
}

impl RpmBuilder {
    pub fn new(build_root: &str) -> Self {
        Self { build_root: RpmBuildRoot::new(&build_root) }
    }

    pub fn write_patch_info_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let rpm_source_dir = self.build_root.get_source_path();
        let patch_info_file_path = format!("{}/{}", rpm_source_dir, PATCH_INFO_FILE_NAME);

        fs::write_string_to_file(
            patch_info_file_path,
            &patch_info.to_string()
        )
    }

    pub fn write_patch_target_info_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let rpm_source_dir = self.build_root.get_source_path();
        let version_file_path = format!("{}/{}", rpm_source_dir, PKG_PATCH_VERSION_FILE_NAME);
        let target_file_path  = format!("{}/{}", rpm_source_dir, PKG_PATCH_TARGET_FILE_NAME);

        fs::write_string_to_file(
            version_file_path,
            patch_info.get_patch().get_version()
        )?;
        fs::write_string_to_file(
            target_file_path,
            patch_info.get_target().unwrap().to_string().as_str()
        )?;

        Ok(())
    }

    pub fn copy_patch_file_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        for patch_file in patch_info.get_file_list() {
            let src_path = patch_file.get_path();
            fs::check_file(src_path)?;

            let dst_path = format!("{}/{}", self.build_root.get_source_path(), patch_file.get_name());
            std::fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }

    pub fn copy_all_files_to_source(&self, src_dir: &str) -> std::io::Result<()> {
        fs::copy_all_files(src_dir, self.build_root.get_source_path())
    }

    pub fn generate_spec_file(&self, patch_info: &PatchInfo) -> std::io::Result<String> {
        RpmSpecGenerator::generate_from_patch_info(
            patch_info,
            self.build_root.get_source_path(),
            self.build_root.get_spec_path()
        )
    }

    pub fn build_source_package(&self, spec_file_path: &str, output_dir: &str) -> std::io::Result<()> {
        fs::check_file(spec_file_path)?;
        fs::check_dir(output_dir)?;

        // Build source rpm
        let exit_status = RPM_BUILD.execvp([
            "--define", &format!("_topdir {}", self.build_root),
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
