use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::constants::*;
use crate::util::fs;

use crate::patch::PatchInfo;
use crate::util::os_str::OsStrConcat;
use crate::workdir::PackageBuildRoot;
use crate::cmd::ExternCommandArgs;

use super::rpm_helper::RpmHelper;
use super::rpm_spec_generator::RpmSpecGenerator;

pub struct RpmBuilder {
    build_root: PackageBuildRoot
}

impl RpmBuilder {
    pub fn new(build_root: PackageBuildRoot) -> Self {
        Self { build_root }
    }

    pub fn write_patch_info_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let rpm_source_dir = self.build_root.sources_dir();
        let patch_info_file_path = rpm_source_dir.join(PATCH_INFO_FILE_NAME);

        fs::write_string_to_file(
            patch_info_file_path,
            format!("{}\n", patch_info).as_str()
        )
    }

    pub fn write_patch_target_info_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        let rpm_source_dir = self.build_root.sources_dir();
        let version_file_path = rpm_source_dir.join(PKG_VERSION_FILE_NAME);
        let target_file_path  = rpm_source_dir.join(PKG_TARGET_FILE_NAME);
        fs::write_string_to_file(
            version_file_path,
            patch_info.get_version()
        )?;
        fs::write_string_to_file(
            target_file_path,
            &patch_info.get_target().to_query_str()
        )?;

        Ok(())
    }

    pub fn copy_patch_file_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        for patch_file in patch_info.get_file_list() {
            let src_path = patch_file.get_path();
            fs::check_file(src_path)?;

            let dst_path = self.build_root.sources_dir().join(patch_file.get_name());
            std::fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }

    pub fn copy_all_files_to_source<P: AsRef<Path>>(&self, src_dir: P) -> std::io::Result<()> {
        fs::copy_all_files(src_dir, self.build_root.sources_dir())
    }

    pub fn generate_spec_file(&self, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        RpmSpecGenerator::generate_from_patch_info(
            patch_info,
            self.build_root.sources_dir(),
            self.build_root.specs_dir()
        )
    }

    pub fn build_prepare(&self) -> std::io::Result<()> {
        let spec_file_path = RpmHelper::find_spec_file(self.build_root.specs_dir())?;

        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bp")
                .arg(spec_file_path)
        )?;
        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        Ok(())
    }

    pub fn build_source_package<P: AsRef<Path>>(&self, output_dir: P) -> std::io::Result<()> {
        fs::check_dir(&output_dir)?;

        let spec_file_path = RpmHelper::find_spec_file(self.build_root.specs_dir())?;
        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bs")
                .arg(spec_file_path)
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        fs::copy_all_files(self.build_root.srpms_dir(), &output_dir)?;

        Ok(())
    }

    pub fn build_binary_package<P: AsRef<Path>>(&self, output_dir: P) -> std::io::Result<()> {
        fs::check_dir(&output_dir)?;

        let spec_file_path = RpmHelper::find_spec_file(self.build_root.specs_dir())?;
        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bb")
                .arg(spec_file_path)
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        fs::copy_all_files(self.build_root.rpms_dir(), &output_dir)?;

        Ok(())
    }
}
