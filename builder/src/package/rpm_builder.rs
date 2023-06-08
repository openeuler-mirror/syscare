use std::ffi::OsString;
use std::path::{Path, PathBuf};

use common::util::ext_cmd::ExternCommandArgs;
use common::util::fs;
use common::util::os_str::OsStringExt;

use crate::patch::PatchInfo;
use crate::workdir::PackageBuildRoot;

use super::rpm_helper::{PKG_FILE_EXT, RPM_BUILD};
use super::rpm_spec_generator::RpmSpecGenerator;

pub struct RpmBuilder {
    build_root: PackageBuildRoot,
}

impl RpmBuilder {
    pub fn new(build_root: PackageBuildRoot) -> Self {
        Self { build_root }
    }

    pub fn build_root(&self) -> &PackageBuildRoot {
        &self.build_root
    }

    pub fn generate_spec_file(&self, patch_info: &PatchInfo) -> std::io::Result<PathBuf> {
        RpmSpecGenerator::generate_spec_file(
            patch_info,
            &self.build_root.sources,
            &self.build_root.specs,
        )
    }

    pub fn build_prepare<P: AsRef<Path>>(&self, spec_file: P) -> std::io::Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("-bp")
                    .arg(spec_file.as_ref()),
            )?
            .check_exit_code()
    }

    pub fn build_source_package<P, Q>(
        &self,
        patch_info: &PatchInfo,
        spec_file: P,
        output_dir: Q,
    ) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir ").concat(self.build_root.as_ref()))
                    .arg("-bs")
                    .arg(spec_file.as_ref()),
            )?
            .check_exit_code()?;

        let src_pkg_file = fs::find_file_by_ext(
            &self.build_root.srpms,
            PKG_FILE_EXT,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;

        let dst_pkg_name = format!(
            "{}-{}.src.{}",
            patch_info.target.short_name(),
            patch_info.full_name(),
            PKG_FILE_EXT
        );
        let dst_pkg_file = output_dir.as_ref().join(dst_pkg_name);

        fs::copy(src_pkg_file, dst_pkg_file)?;

        Ok(())
    }

    pub fn build_binary_package<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        spec_file: P,
        output_dir: Q,
    ) -> std::io::Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("-bb")
                    .arg(spec_file.as_ref()),
            )?
            .check_exit_code()?;

        fs::copy_dir_contents(&self.build_root.rpms, &output_dir)?;

        Ok(())
    }
}
