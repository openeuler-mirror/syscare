use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::constants::*;
use crate::util::{fs, serde};
use crate::util::os_str::OsStrConcat;

use crate::patch::PatchInfo;
use crate::workdir::PackageBuildRoot;
use crate::cmd::ExternCommandArgs;

use super::rpm_spec_generator::RpmSpecGenerator;

pub struct RpmBuilder {
    build_root: PackageBuildRoot
}

impl RpmBuilder {
    pub fn new(build_root: PackageBuildRoot) -> Self {
        Self { build_root }
    }

    pub fn write_patch_info_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        serde::serialize(
            patch_info,
            self.build_root.sources.join(PATCH_INFO_FILE_NAME)
        )
    }

    pub fn copy_patch_file_to_source(&self, patch_info: &PatchInfo) -> std::io::Result<()> {
        for patch_file in &patch_info.patches {
            let src_path = patch_file.path.as_path();
            fs::check_file(src_path)?;

            let dst_path = self.build_root.sources.join(&patch_file.name);
            std::fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }

    pub fn copy_all_files_to_source<P: AsRef<Path>>(&self, src_dir: P) -> std::io::Result<()> {
        fs::copy_all_files(src_dir, &self.build_root.sources)
    }

    pub fn generate_spec_file(&self, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        RpmSpecGenerator::generate_from_patch_info(
            patch_info,
            &self.build_root.sources,
            &self.build_root.specs
        )
    }

    pub fn build_prepare<P: AsRef<Path>>(&self, spec_file: P) -> std::io::Result<()> {
        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bp")
                .arg(spec_file.as_ref())
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

    pub fn build_source_package<P: AsRef<Path>, Q: AsRef<Path>>(&self, spec_file: P, output_dir: Q) -> std::io::Result<()> {
        fs::check_dir(&output_dir)?;

        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bs")
                .arg(spec_file.as_ref())
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        fs::copy_all_files(&self.build_root.srpms, &output_dir)?;

        Ok(())
    }

    pub fn build_binary_package<P: AsRef<Path>, Q: AsRef<Path>>(&self, spec_file: P, output_dir: Q) -> std::io::Result<()> {
        fs::check_dir(&output_dir)?;

        let exit_status = RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bb")
                .arg(spec_file.as_ref())
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM_BUILD, exit_code),
            ));
        }

        fs::copy_all_files(&self.build_root.rpms, &output_dir)?;

        Ok(())
    }
}
