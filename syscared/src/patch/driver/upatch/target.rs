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
    collections::{hash_map::Entry, HashMap, HashSet},
    path::PathBuf,
};

use indexmap::IndexSet;
use uuid::Uuid;

use crate::patch::entity::UserPatch;

#[derive(Debug, Default)]
pub struct PatchTarget {
    process_list: HashSet<i32>,
    patch_map: HashMap<Uuid, PathBuf>, // uuid -> patch file
    collision_map: HashMap<u64, IndexSet<Uuid>>, // function old addr -> patch collision list
}

impl PatchTarget {
    pub fn add_process(&mut self, pid: i32) {
        self.process_list.insert(pid);
    }

    pub fn remove_process(&mut self, pid: i32) {
        self.process_list.remove(&pid);
    }

    pub fn clean_dead_process(&mut self, process_list: &HashSet<i32>) {
        self.process_list.retain(|pid| process_list.contains(pid));
    }

    pub fn need_actived(&self, process_list: &HashSet<i32>) -> HashSet<i32> {
        process_list
            .difference(&self.process_list)
            .copied()
            .collect()
    }

    pub fn need_deactived(&self, process_list: &HashSet<i32>) -> HashSet<i32> {
        process_list
            .intersection(&self.process_list)
            .copied()
            .collect()
    }
}

impl PatchTarget {
    pub fn add_patch(&mut self, patch: &UserPatch) {
        for function in &patch.functions {
            self.collision_map
                .entry(function.old_addr)
                .or_default()
                .insert(patch.uuid);
        }
        self.patch_map.insert(patch.uuid, patch.patch_file.clone());
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
        self.patch_map.remove(&patch.uuid);
    }

    pub fn is_patched(&self) -> bool {
        !self.collision_map.is_empty()
    }

    pub fn all_patches(&self) -> impl Iterator<Item = (Uuid, PathBuf)> + '_ {
        self.patch_map
            .iter()
            .map(|(uuid, path)| (*uuid, path.to_path_buf()))
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
