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

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use syscare_abi::PatchEntity;
use syscare_common::util::os_str::OsStrExt;

use super::PatchInfoExt;

#[derive(Debug)]
pub struct KernelPatchExt {
    pub patch_file: PathBuf,
    pub sys_file: PathBuf,
}

impl KernelPatchExt {
    pub fn new<P: AsRef<Path>>(patch_root: P, patch_entity: &PatchEntity) -> Self {
        const KPATCH_SUFFIX: &str = ".ko";
        const KPATCH_MGNT_DIR: &str = "/sys/kernel/livepatch";
        const KPATCH_MGNT_FILE_NAME: &str = "enabled";

        let patch_name = patch_entity
            .patch_name
            .strip_suffix(KPATCH_SUFFIX)
            .map(OsStr::to_string_lossy)
            .unwrap_or_else(|| patch_entity.patch_name.to_string_lossy());
        let patch_sys_name = patch_name.replace(['-', '.'], "_");
        let patch_file_name = format!("{}{}", patch_name, KPATCH_SUFFIX);

        Self {
            patch_file: patch_root.as_ref().join(patch_file_name),
            sys_file: PathBuf::from(KPATCH_MGNT_DIR)
                .join(patch_sys_name)
                .join(KPATCH_MGNT_FILE_NAME),
        }
    }
}

impl<'a> From<&'a PatchInfoExt> for &'a KernelPatchExt {
    fn from(ext: &'a PatchInfoExt) -> Self {
        match ext {
            PatchInfoExt::KernelPatch(ext) => ext,
            _ => panic!("Cannot convert user patch ext into kernel patch ext"),
        }
    }
}
