use std::{ffi::OsString, path::Path};

use anyhow::{Context, Result};

use syscare_common::util::{
    ext_cmd::{ExternCommand, ExternCommandArgs},
    fs,
    os_str::OsStringExt,
};

use super::PKG_FILE_EXT;
use crate::{
    build_params::BuildParameters,
    package::{PackageBuildRoot, PackageBuilder},
};

const RPM_BUILD: ExternCommand = ExternCommand::new("rpmbuild");

pub struct RpmPackageBuilder<'a> {
    build_root: &'a PackageBuildRoot,
}

impl<'a> RpmPackageBuilder<'a> {
    pub fn new(build_root: &'a PackageBuildRoot) -> Self {
        Self { build_root }
    }
}

impl PackageBuilder for RpmPackageBuilder<'_> {
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
        build_params: &BuildParameters,
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
            "{}-{}-{}-{}.src.{}",
            build_params.patch.target.short_name(),
            build_params.patch.name,
            build_params.patch.version,
            build_params.patch.release,
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