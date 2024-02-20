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
use syscare_abi::PatchInfo;

use super::{rpm::RpmSpecBuilder, PackageFormat};

pub trait PackageSpecBuilder {
    fn build(
        &self,
        patch_info: &PatchInfo,
        patch_requires: &[String],
        source_dir: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf>;
}

pub struct PackageSpecBuilderFactory;

impl PackageSpecBuilderFactory {
    pub fn get_builder(pkg_format: PackageFormat) -> Box<dyn PackageSpecBuilder> {
        match pkg_format {
            PackageFormat::RpmPackage => Box::new(RpmSpecBuilder),
        }
    }
}
