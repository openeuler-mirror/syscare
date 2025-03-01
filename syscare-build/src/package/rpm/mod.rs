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

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use syscare_abi::{PackageInfo, PackageType};
use syscare_common::{ffi::OsStrExt, fs, process::Command};

mod pkg_builder;
mod spec_builder;
mod spec_file;
mod spec_writer;
mod tags;

pub use pkg_builder::RpmPackageBuilder;
pub use spec_builder::RpmSpecBuilder;
pub use spec_writer::RpmSpecWriter;

use super::{Package, PackageBuildRoot};

pub const PKG_FILE_EXT: &str = "rpm";
pub const SPEC_FILE_EXT: &str = "spec";
pub const SPEC_TAG_VALUE_NONE: &str = "(none)";
pub const SPEC_SCRIPT_VALUE_NONE: &str = "# None";

const RPM_BIN: &str = "rpm";
const PKG_BUILD_ROOT: &str = "rpmbuild";

pub struct RpmPackage;

impl RpmPackage {
    fn query_package_info<P: AsRef<Path>>(pkg_path: P, format: &str) -> Result<OsString> {
        let output = Command::new(RPM_BIN)
            .arg("--query")
            .arg("--queryformat")
            .arg(format)
            .arg("--nosignature")
            .arg("--package")
            .arg(pkg_path.as_ref().as_os_str())
            .run_with_output()?;
        output.exit_ok()?;

        Ok(output.stdout)
    }
}

impl Package for RpmPackage {
    fn parse_package_info(&self, pkg_path: &Path) -> Result<PackageInfo> {
        let query_result = Self::query_package_info(
            pkg_path,
            "%{NAME}|%{ARCH}|%{EPOCH}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}",
        )?
        .to_string_lossy()
        .to_string();

        let pkg_info = query_result.split('|').collect::<Vec<_>>();
        if pkg_info.len() < 7 {
            bail!("Parse package info from {} failed", pkg_path.display());
        }

        let name = pkg_info[0].to_owned();
        let kind = if pkg_info[6] == SPEC_TAG_VALUE_NONE {
            PackageType::SourcePackage
        } else {
            PackageType::BinaryPackage
        };
        let arch = pkg_info[1].to_owned();
        let epoch = pkg_info[2].to_owned();
        let version = pkg_info[3].to_owned();
        let release = pkg_info[4].to_owned();
        let license = pkg_info[5].to_owned();
        let source_pkg = match kind {
            // For source package, it doesn't have %SOURCERPM, we reuse this field to store file name
            PackageType::SourcePackage => fs::file_name(pkg_path).to_string_lossy().to_string(),
            PackageType::BinaryPackage => pkg_info[6].to_owned(),
        };

        Ok(PackageInfo {
            name,
            kind,
            arch,
            epoch,
            version,
            release,
            license,
            source_pkg,
        })
    }

    fn query_package_files(&self, pkg_path: &Path) -> Result<Vec<PathBuf>> {
        let output = Command::new(RPM_BIN)
            .arg("--query")
            .arg("--list")
            .arg("--nosignature")
            .arg("--package")
            .arg(pkg_path)
            .run_with_output()?;
        output.exit_ok()?;

        let mut file_list = Vec::new();
        for file in output.stdout.split('\n') {
            file_list.push(PathBuf::from(file));
        }

        Ok(file_list)
    }

    fn extract_package(&self, pkg_path: &Path, output_dir: &Path) -> Result<()> {
        Command::new(RPM_BIN)
            .arg("--install")
            .arg("--nodeps")
            .arg("--nofiledigest")
            .arg("--nocontexts")
            .arg("--nocaps")
            .arg("--noscripts")
            .arg("--notriggers")
            .arg("--nodigest")
            .arg("--nofiledigest")
            .arg("--allfiles")
            .arg("--root")
            .arg(output_dir)
            .arg("--package")
            .arg(pkg_path)
            .run()?
            .exit_ok()
    }

    fn find_build_root(&self, directory: &Path) -> Result<PackageBuildRoot> {
        let build_root = fs::find_dir(
            directory,
            PKG_BUILD_ROOT,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )?;
        PackageBuildRoot::new(build_root)
    }

    fn find_spec_file(&self, directory: &Path, package_name: &str) -> Result<PathBuf> {
        let file_name = format!("{}.{}", package_name, SPEC_FILE_EXT);
        let spec_file = fs::find_file(
            directory,
            file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;
        Ok(spec_file)
    }

    fn find_source_directory(&self, directory: &Path, package_name: &str) -> Result<PathBuf> {
        let build_source = fs::find_dir(
            directory,
            package_name,
            fs::FindOptions {
                fuzz: true,
                recursive: true,
            },
        )
        .or_else(|_| {
            fs::find_dir(
                directory,
                "",
                fs::FindOptions {
                    fuzz: true,
                    recursive: true,
                },
            )
        })?;

        Ok(build_source)
    }
}
