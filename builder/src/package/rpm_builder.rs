use std::{ffi::OsString, path::Path};

use anyhow::{Context, Result};
use syscare_abi::PatchInfo;
use syscare_common::util::{ext_cmd::ExternCommandArgs, fs, os_str::OsStringExt};

use crate::workdir::RpmBuildRoot;

use super::rpm_helper::{PKG_FILE_EXT, RPM_BUILD};

pub struct RpmBuilder {
    build_root: RpmBuildRoot,
}

impl RpmBuilder {
    pub fn new(build_root: RpmBuildRoot) -> Self {
        Self { build_root }
    }

    pub fn build_prepare<P: AsRef<Path>>(&self, spec_file: P) -> Result<()> {
        Ok(RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("-bp")
                    .arg(spec_file.as_ref()),
            )?
            .check_exit_code()?)
    }

    pub fn build_source_package<P, Q>(
        &self,
        patch_info: &PatchInfo,
        spec_file: P,
        output_dir: Q,
    ) -> Result<()>
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
        let dst_pkg_file = output_dir.as_ref().join(dst_pkg_name);

        fs::copy(src_pkg_file, dst_pkg_file).context("Cannot copy package to output directory")?;

        Ok(())
    }

    pub fn build_binary_package<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        spec_file: P,
        output_dir: Q,
    ) -> Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("--define")
                    .arg("__spec_install_post %{nil}")
                    .arg("-bb")
                    .arg(spec_file.as_ref()),
            )?
            .check_exit_code()?;

        fs::copy_dir_contents(&self.build_root.rpms, &output_dir)
            .context("Cannot copy package to output directory")?;

        Ok(())
    }
}
