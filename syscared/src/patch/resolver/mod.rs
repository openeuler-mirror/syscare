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

use std::{path::Path, sync::Arc};

use super::{entity::Patch, PATCH_INFO_FILE_NAME};

use anyhow::{Context, Result};
use syscare_abi::{PatchEntity, PatchInfo, PatchType, PATCH_INFO_MAGIC};
use syscare_common::util::serde;

mod kpatch;
mod upatch;

use kpatch::KpatchResolverImpl;
use upatch::UpatchResolverImpl;

pub trait PatchResolverImpl {
    fn resolve_patch(
        &self,
        patch_root: &Path,
        patch_info: Arc<PatchInfo>,
        patch_entity: &PatchEntity,
    ) -> Result<Patch>;
}

pub struct PatchResolver;

impl PatchResolver {
    pub fn resolve_patch<P: AsRef<Path>>(patch_root: P) -> Result<Vec<Patch>> {
        let patch_root = patch_root.as_ref();
        let patch_info = Arc::new(
            serde::deserialize_with_magic::<PatchInfo, _, _>(
                patch_root.join(PATCH_INFO_FILE_NAME),
                PATCH_INFO_MAGIC,
            )
            .context("Failed to resolve patch metadata")?,
        );
        let resolver = match patch_info.kind {
            PatchType::UserPatch => &UpatchResolverImpl as &dyn PatchResolverImpl,
            PatchType::KernelPatch => &KpatchResolverImpl as &dyn PatchResolverImpl,
        };

        let mut patch_list = Vec::with_capacity(patch_info.entities.len());
        for patch_entity in &patch_info.entities {
            let patch = resolver.resolve_patch(patch_root, patch_info.clone(), patch_entity)?;
            patch_list.push(patch);
        }

        Ok(patch_list)
    }
}
