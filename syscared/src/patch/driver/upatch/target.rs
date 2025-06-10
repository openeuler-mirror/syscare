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

use std::collections::{hash_map::Entry, HashMap};

use indexmap::IndexSet;
use uuid::Uuid;

use crate::patch::entity::UserPatch;

#[derive(Debug, Default)]
pub struct PatchTarget {
    collision_map: HashMap<u64, IndexSet<Uuid>>, // function old addr -> patch collision list
}

impl PatchTarget {
    pub fn add_patch(&mut self, patch: &UserPatch) {
        for function in &patch.functions {
            self.collision_map
                .entry(function.old_addr)
                .or_default()
                .insert(patch.uuid);
        }
    }

    pub fn remove_patch(&mut self, patch: &UserPatch) {
        for function in &patch.functions {
            if let Entry::Occupied(mut entry) = self.collision_map.entry(function.old_addr) {
                let patch_set = entry.get_mut();
                patch_set.shift_remove(&patch.uuid);

                if patch_set.is_empty() {
                    entry.remove();
                }
            }
        }
    }

    pub fn is_patched(&self) -> bool {
        !self.collision_map.is_empty()
    }

    pub fn get_conflicted_patches<'a>(
        &'a self,
        patch: &'a UserPatch,
    ) -> impl Iterator<Item = Uuid> + 'a {
        patch
            .functions
            .iter()
            .filter_map(move |function| self.collision_map.get(&function.old_addr))
            .flatten()
            .copied()
            .filter(move |&uuid| uuid != patch.uuid)
    }

    pub fn get_overridden_patches<'a>(
        &'a self,
        patch: &'a UserPatch,
    ) -> impl Iterator<Item = Uuid> + 'a {
        patch
            .functions
            .iter()
            .filter_map(move |function| self.collision_map.get(&function.old_addr))
            .flat_map(move |collision_list| {
                collision_list
                    .iter()
                    .copied()
                    .skip_while(move |&uuid| uuid != patch.uuid)
                    .skip(1)
            })
    }
}
