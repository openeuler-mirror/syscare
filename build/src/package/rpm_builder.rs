use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::constants::*;

use crate::patch::PatchInfo;
use crate::workdir::PackageBuildRoot;
use crate::util::{fs, serde};
use crate::util::os_str::OsStrConcat;
use crate::util::ext_cmd::ExternCommandArgs;

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
            let dst_path = self.build_root.sources.join(&patch_file.name);

            fs::copy(src_path, dst_path)?;
        }

        Ok(())
    }

    pub fn copy_all_files_to_source<P: AsRef<Path>>(&self, src_dir: P) -> std::io::Result<()> {
        fs::copy_dir_all(src_dir, &self.build_root.sources)
    }

    pub fn generate_spec_file(&self, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        RpmSpecGenerator::generate_from_patch_info(
            patch_info,
            &self.build_root.sources,
            &self.build_root.specs
        )
    }

    pub fn build_prepare<P: AsRef<Path>>(&self, spec_file: P) -> std::io::Result<()> {
        RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bp")
                .arg(spec_file.as_ref())
        )?.check_exit_code()
    }

    pub fn build_source_package<P, Q>(&self, patch_info: &PatchInfo, spec_file: P, output_dir: Q) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bs")
                .arg(spec_file.as_ref())
        )?.check_exit_code()?;

        let src_pkg_file = fs::find_file_ext(
            &self.build_root.srpms,
            PKG_FILE_EXTENSION,
            false
        )?;

        let dst_pkg_name = format!("{}-{}.src.{}",
            patch_info.target.short_name(),
            patch_info.full_name(),
            PKG_FILE_EXTENSION
        );
        let dst_pkg_file = output_dir.as_ref().join(dst_pkg_name);

        fs::copy(src_pkg_file, dst_pkg_file)?;

        Ok(())
    }

    pub fn build_binary_package<P: AsRef<Path>, Q: AsRef<Path>>(&self, spec_file: P, output_dir: Q) -> std::io::Result<()> {
        RPM_BUILD.execvp(
            ExternCommandArgs::new()
                .arg("--define")
                .arg(OsString::from("_topdir ").concat(&self.build_root))
                .arg("-bb")
                .arg(spec_file.as_ref())
        )?.check_exit_code()?;

        fs::copy_dir_all(&self.build_root.rpms, &output_dir)?;

        Ok(())
    }
}
