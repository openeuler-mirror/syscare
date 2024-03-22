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

use std::path::{Path, PathBuf};

use anyhow::Result;

use syscare_abi::PackageInfo;

mod build_root;
mod pkg_builder;
mod spec_builder;
mod spec_writer;

mod rpm;
mod tar;

pub use build_root::*;
pub use pkg_builder::*;
pub use spec_builder::*;
pub use spec_writer::*;
pub use tar::*;

trait Package {
    fn extension(&self) -> &'static str;
    fn parse_package_info(&self, pkg_path: &Path) -> Result<PackageInfo>;
    fn query_package_files(&self, pkg_path: &Path) -> Result<Vec<PathBuf>>;
    fn extract_package(&self, pkg_path: &Path, output_dir: &Path) -> Result<()>;
    fn find_build_root(&self, directory: &Path) -> Result<PackageBuildRoot>;
    fn find_spec_file(&self, directory: &Path, pkg_name: &str) -> Result<PathBuf>;
    fn find_source_directory(&self, directory: &Path, pkg_name: &str) -> Result<PathBuf>;
}

#[derive(Debug, Clone, Copy)]
pub enum PackageFormat {
    RpmPackage,
}

pub struct PackageImpl {
    format: PackageFormat,
    inner: Box<dyn Package + Send + Sync>,
}

impl PackageImpl {
    pub fn new(pkg_format: PackageFormat) -> Self {
        match pkg_format {
            PackageFormat::RpmPackage => Self {
                format: pkg_format,
                inner: Box::new(rpm::RpmPackage),
            },
        }
    }

    pub fn format(&self) -> PackageFormat {
        self.format
    }

    pub fn parse_package_info<P: AsRef<Path>>(&self, pkg_path: P) -> Result<PackageInfo> {
        self.inner.parse_package_info(pkg_path.as_ref())
    }

    pub fn query_package_files<P: AsRef<Path>>(&self, pkg_path: P) -> Result<Vec<PathBuf>> {
        self.inner.query_package_files(pkg_path.as_ref())
    }

    pub fn extract_package<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        pkg_path: P,
        output_dir: Q,
    ) -> Result<()> {
        self.inner
            .extract_package(pkg_path.as_ref(), output_dir.as_ref())
    }

    pub fn find_build_root<P: AsRef<Path>>(&self, directory: P) -> Result<PackageBuildRoot> {
        self.inner.find_build_root(directory.as_ref())
    }

    pub fn find_spec_file<P: AsRef<Path>>(&self, directory: P, pkg_name: &str) -> Result<PathBuf> {
        self.inner.find_spec_file(directory.as_ref(), pkg_name)
    }

    pub fn find_source_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        pkg_name: &str,
    ) -> Result<PathBuf> {
        self.inner
            .find_source_directory(directory.as_ref(), pkg_name)
    }
}
