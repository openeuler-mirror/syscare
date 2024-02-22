// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
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

use syscare_abi::PatchEntity;

use super::PatchInfoExt;

#[derive(Debug)]
pub struct UserPatchExt {
    pub patch_file: PathBuf,
    pub target_elf: PathBuf,
}

impl UserPatchExt {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_entity: &PatchEntity) -> Self {
        Self {
            patch_file: patch_root
                .as_ref()
                .join(patch_entity.patch_name.as_os_str()),
            target_elf: patch_entity.patch_target.to_path_buf(),
        }
    }
}

impl<'a> From<&'a PatchInfoExt> for &'a UserPatchExt {
    fn from(ext: &'a PatchInfoExt) -> Self {
        match ext {
            PatchInfoExt::UserPatch(ext) => ext,
            _ => panic!("Cannot convert kernel patch ext into user patch ext"),
        }
    }
}
