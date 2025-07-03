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

use anyhow::{Context, Result};

use uuid::Uuid;

use syscare_abi::{PatchInfo, PATCH_INFO_MAGIC};
use syscare_common::{fs, util::serde};

use crate::{build_params::BuildParameters, package::TarPackage};

const METADATA_DIR_NAME: &str = ".syscare";
const METADATA_PKG_NAME: &str = ".syscare.tar.gz";
const METADATA_FILE_NAME: &str = "patch_info";

pub struct PatchMetadata {
    root_dir: PathBuf,
    pub metadata_dir: PathBuf,
    pub package_path: PathBuf,
    pub metadata_path: PathBuf,
}

impl PatchMetadata {
    pub fn new<P: AsRef<Path>>(directory: P) -> Self {
        let root_dir = directory.as_ref().to_path_buf();
        let metadata_dir = root_dir.join(METADATA_DIR_NAME);
        let package_path = root_dir.join(METADATA_PKG_NAME);
        let metadata_path = metadata_dir.join(METADATA_FILE_NAME);

        Self {
            root_dir,
            metadata_dir,
            package_path,
            metadata_path,
        }
    }

    pub fn create(&self, build_params: &BuildParameters) -> Result<&Path> {
        if !self.metadata_dir.exists() {
            fs::create_dir_all(&self.metadata_dir)?;
        }

        for patch in &build_params.patch_files {
            let src_path = &patch.path;
            let dst_path = self.metadata_dir.join(&patch.name);
            if src_path != &dst_path {
                fs::copy(src_path, dst_path).context("Failed to copy patch files")?;
            }
        }

        let patch_info = PatchInfo {
            uuid: Uuid::default(),
            name: build_params.patch_name.to_owned(),
            version: build_params.patch_version.to_owned(),
            release: build_params.patch_release.to_owned(),
            arch: build_params.patch_arch.to_owned(),
            kind: build_params.patch_type,
            target: build_params.build_entry.target_pkg.to_owned(),
            entities: Vec::default(),
            description: build_params.patch_description.to_owned(),
            patches: build_params.patch_files.to_owned(),
        };

        self.write(&patch_info, &self.metadata_dir)
            .context("Failed to write patch metadata")?;

        TarPackage::new(&self.package_path)
            .compress(&self.root_dir, METADATA_DIR_NAME)
            .context("Failed to compress patch metadata")?;

        Ok(&self.package_path)
    }

    pub fn extract(&self) -> Result<PatchInfo> {
        TarPackage::new(&self.package_path)
            .decompress(&self.root_dir)
            .context("Failed to decompress patch metadata")?;

        let mut patch_info: PatchInfo =
            serde::deserialize_with_magic(&self.metadata_path, PATCH_INFO_MAGIC)
                .context("Failed to read patch metadata")?;

        // rewrite file path to metadata directory path
        for patch_file in &mut patch_info.patches {
            patch_file.path = self.metadata_dir.join(&patch_file.name)
        }

        Ok(patch_info)
    }

    pub fn write<P: AsRef<Path>>(
        &self,
        patch_info: &PatchInfo,
        output_dir: P,
    ) -> std::io::Result<()> {
        serde::serialize_with_magic(
            patch_info,
            output_dir.as_ref().join(METADATA_FILE_NAME),
            PATCH_INFO_MAGIC,
        )
    }
}
