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

use std::{collections::HashSet, path::Path};

use anyhow::{bail, Result};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use syscare_abi::PatchFile;
use syscare_common::{fs, util::digest};

pub struct PatchHelper;

impl PatchHelper {
    pub fn collect_patch_files<I, P>(patch_files: I) -> Result<Vec<PatchFile>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        lazy_static! {
            static ref FILE_DIGESTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        let mut patch_list = Vec::new();
        for patch_file in patch_files {
            let file_path = fs::canonicalize(patch_file)?;
            let file_name = fs::file_name(&file_path);
            let file_digest = digest::file(&file_path)?;

            if !FILE_DIGESTS.lock().insert(file_digest.clone()) {
                bail!("Patch {} is duplicated", file_path.display());
            }
            patch_list.push(PatchFile {
                name: file_name,
                path: file_path,
                digest: file_digest,
            });
        }
        patch_list.sort();
        Ok(patch_list)
    }
}
