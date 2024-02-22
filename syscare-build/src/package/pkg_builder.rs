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

use std::path::Path;

use anyhow::Result;

use crate::BuildParameters;

use super::{rpm::RpmPackageBuilder, PackageBuildRoot, PackageFormat};

pub trait PackageBuilder {
    fn build_prepare(&self, spec_file: &Path) -> Result<()>;
    fn build_source_package(
        &self,
        build_params: &BuildParameters,
        spec_file: &Path,
        output_dir: &Path,
    ) -> Result<()>;
    fn build_binary_package(&self, spec_file: &Path, output_dir: &Path) -> Result<()>;
}

pub struct PackageBuilderFactory;

impl PackageBuilderFactory {
    pub fn get_builder(
        pkg_format: PackageFormat,
        build_root: &PackageBuildRoot,
    ) -> Box<dyn PackageBuilder + '_> {
        match pkg_format {
            PackageFormat::RpmPackage => Box::new(RpmPackageBuilder::new(build_root)),
        }
    }
}
