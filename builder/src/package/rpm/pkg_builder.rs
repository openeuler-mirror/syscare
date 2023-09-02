use std::{ffi::OsString, path::Path};

use anyhow::{Context, Result};
use syscare_abi::PatchInfo;
use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    fs,
    os_str::OsStringExt,
};

const RPM_BUILD: ExternCommand = ExternCommand::new("rpmbuild");

use super::PKG_FILE_EXT;
use crate::package::{PackageBuildRoot, PackageBuilder};

pub struct RpmPackageBuilder {
    build_root: PackageBuildRoot,
}

impl RpmPackageBuilder {
    pub fn new(build_root: PackageBuildRoot) -> Self {
        Self { build_root }
    }
}

impl PackageBuilder for RpmPackageBuilder {
    fn build_prepare(&self, spec_file: &Path) -> Result<()> {
        Ok(RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("-bp")
                    .arg(spec_file),
            )?
            .check_exit_code()?)
    }

    fn build_source_package(
        &self,
        patch_info: &PatchInfo,
        spec_file: &Path,
        output_dir: &Path,
    ) -> Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir ").concat(self.build_root.as_ref()))
                    .arg("-bs")
                    .arg(spec_file),
            )?
            .check_exit_code()?;

        let srpms_dir = &self.build_root.srpms;
        let src_pkg_file = fs::find_file_by_ext(
            srpms_dir,
            PKG_FILE_EXT,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )
        .with_context(|| {
            format!(
                "Cannot find source package from \"{}\"",
                srpms_dir.display()
            )
        })?;

        let dst_pkg_name = format!(
            "{}-{}.src.{}",
            patch_info.target.short_name(),
            patch_info.name(),
            PKG_FILE_EXT
        );
        let dst_pkg_file = output_dir.join(dst_pkg_name);

        fs::copy(src_pkg_file, dst_pkg_file).context("Cannot copy package to output directory")?;

        Ok(())
    }

    fn build_binary_package(&self, spec_file: &Path, output_dir: &Path) -> Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("--define")
                    .arg("__spec_install_post %{nil}")
                    .arg("-bb")
                    .arg(spec_file),
            )?
            .check_exit_code()?;

        fs::copy_dir_contents(&self.build_root.rpms, output_dir)
            .context("Cannot copy package to output directory")?;

        Ok(())
    }
}
