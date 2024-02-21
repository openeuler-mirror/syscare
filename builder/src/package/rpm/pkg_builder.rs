// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{ffi::OsString, path::Path};

use anyhow::{Context, Result};

use lazy_static::lazy_static;
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

lazy_static! {
    static ref RPM_BUILD: ExternCommand = ExternCommand::new("rpmbuild");
}

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
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("-bp")
                    .arg(spec_file),
            )?
            .check_exit_code()
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
            build_params.build_entry.target_pkg.short_name(),
            build_params.patch_name,
            build_params.patch_version,
            build_params.patch_release,
            PKG_FILE_EXT
        );
        let dst_pkg_file = output_dir.join(dst_pkg_name);

        fs::copy(src_pkg_file, dst_pkg_file)
            .context("Failed to copy package to output directory")?;

        Ok(())
    }

    fn build_binary_package(&self, spec_file: &Path, output_dir: &Path) -> Result<()> {
        RPM_BUILD
            .execvp(
                ExternCommandArgs::new()
                    .arg("--define")
                    .arg(OsString::from("_topdir").append(self.build_root.as_ref()))
                    .arg("--define")
                    .arg("debug_package %{nil}")
                    .arg("--define")
                    .arg("__spec_install_post %{__arch_install_post}")
                    .arg("--nocheck")
                    .arg("-bb")
                    .arg(spec_file),
            )?
            .check_exit_code()?;

        fs::copy_dir_contents(&self.build_root.rpms, output_dir)
            .context("Failed to copy package to output directory")?;

        Ok(())
    }
}
