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
    collections::{hash_map::Entry, HashMap},
    ffi::OsString,
};

use indexmap::IndexSet;
use uuid::Uuid;

use crate::patch::entity::KernelPatch;

#[derive(Debug)]
pub struct PatchTarget {
    object_name: OsString,
    collision_map: HashMap<OsString, IndexSet<Uuid>>, // function name -> patch collision list
}

impl PatchTarget {
    pub fn new(object_name: OsString) -> Self {
        Self {
            object_name,
            collision_map: HashMap::new(),
        }
    }
}

impl PatchTarget {
    pub fn add_patch(&mut self, patch: &KernelPatch) {
        if let Some(functions) = patch.functions.get(&self.object_name) {
            for function in functions {
                self.collision_map
                    .entry(function.name.clone())
                    .or_default()
                    .insert(patch.uuid);
            }
        }
    }

    pub fn remove_patch(&mut self, patch: &KernelPatch) {
        if let Some(functions) = patch.functions.get(&self.object_name) {
            for function in functions {
                if let Entry::Occupied(mut entry) = self.collision_map.entry(function.name.clone())
                {
                    let patch_set = entry.get_mut();
                    patch_set.shift_remove(&patch.uuid);

                    if patch_set.is_empty() {
                        entry.remove();
                    }
                }
            }
        }
    }

    pub fn is_patched(&self) -> bool {
        !self.collision_map.is_empty()
    }

    pub fn get_conflicted_patches<'a>(
        &'a self,
        patch: &'a KernelPatch,
    ) -> impl Iterator<Item = Uuid> + 'a {
        let functions = patch.functions.get(&self.object_name).into_iter().flatten();
        functions
            .filter_map(move |function| self.collision_map.get(&function.name))
            .flatten()
            .copied()
            .filter(move |&uuid| uuid != patch.uuid)
    }

    pub fn get_overridden_patches<'a>(
        &'a self,
        patch: &'a KernelPatch,
    ) -> impl Iterator<Item = Uuid> + 'a {
        let functions = patch.functions.get(&self.object_name).into_iter().flatten();
        functions
            .filter_map(move |function| self.collision_map.get(&function.name))
            .flat_map(move |collision_list| {
                collision_list
                    .iter()
                    .copied()
                    .skip_while(move |&uuid| uuid != patch.uuid)
                    .skip(1)
            })
    }
}
