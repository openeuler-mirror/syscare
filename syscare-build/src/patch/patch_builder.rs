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

use anyhow::Result;

use syscare_abi::{PatchInfo, PatchType};

use crate::{build_params::BuildParameters, package::PackageImpl};

use super::{kernel_patch::KernelPatchBuilder, user_patch::UserPatchBuilder};

pub trait PatchBuilder {
    fn build_patch(&self, build_params: &BuildParameters) -> Result<Vec<PatchInfo>>;
}

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_builder(
        pkg_impl: &'static PackageImpl,
        patch_type: PatchType,
    ) -> Box<dyn PatchBuilder> {
        match patch_type {
            PatchType::KernelPatch => Box::new(KernelPatchBuilder::new(pkg_impl)),
            PatchType::UserPatch => Box::new(UserPatchBuilder::new(pkg_impl)),
        }
    }
}
