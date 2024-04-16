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

use std::ffi::OsString;

use indexmap::IndexMap;
use uuid::Uuid;

use crate::patch::entity::KernelPatchFunction;

#[derive(Debug)]
pub struct PatchFunction {
    pub uuid: Uuid,
    pub name: OsString,
    pub size: u64,
}

impl PatchFunction {
    fn new(uuid: Uuid, function: &KernelPatchFunction) -> Self {
        Self {
            uuid,
            name: function.name.to_os_string(),
            size: function.new_size,
        }
    }

    fn is_same_function(&self, uuid: &Uuid, function: &KernelPatchFunction) -> bool {
        (self.uuid == *uuid) && (self.name == function.name) && (self.size == function.new_size)
    }
}

#[derive(Debug)]
pub struct PatchTarget {
    name: OsString,
    function_map: IndexMap<u64, Vec<PatchFunction>>, // function addr -> function collision list
}

impl PatchTarget {
    pub fn new(name: OsString) -> Self {
        Self {
            name,
            function_map: IndexMap::new(),
        }
    }
}

impl PatchTarget {
    pub fn has_function(&self) -> bool {
        self.function_map.is_empty()
    }

    pub fn add_functions<'a, I>(&mut self, uuid: Uuid, functions: I)
    where
        I: IntoIterator<Item = &'a KernelPatchFunction>,
    {
        for function in functions {
            if self.name != function.object {
                continue;
            }
            self.function_map
                .entry(function.old_addr)
                .or_default()
                .push(PatchFunction::new(uuid, function));
        }
    }

    pub fn remove_functions<'a, I>(&mut self, uuid: &Uuid, functions: I)
    where
        I: IntoIterator<Item = &'a KernelPatchFunction>,
    {
        for function in functions {
            if self.name != function.object {
                continue;
            }
            if let Some(collision_list) = self.function_map.get_mut(&function.old_addr) {
                if let Some(index) = collision_list
                    .iter()
                    .position(|patch_function| patch_function.is_same_function(uuid, function))
                {
                    collision_list.remove(index);
                    if collision_list.is_empty() {
                        self.function_map.remove(&function.old_addr);
                    }
                }
            }
        }
    }
}

impl PatchTarget {
    pub fn get_conflicts<'a, I>(
        &'a self,
        functions: I,
    ) -> impl IntoIterator<Item = &'a PatchFunction>
    where
        I: IntoIterator<Item = &'a KernelPatchFunction>,
    {
        functions.into_iter().filter_map(move |function| {
            if self.name != function.object {
                return None;
            }
            self.function_map
                .get(&function.old_addr)
                .and_then(|list| list.last())
        })
    }

    pub fn get_overrides<'a, I>(
        &'a self,
        uuid: &'a Uuid,
        functions: I,
    ) -> impl IntoIterator<Item = &'a PatchFunction>
    where
        I: IntoIterator<Item = &'a KernelPatchFunction>,
    {
        functions.into_iter().filter_map(move |function| {
            if self.name != function.object {
                return None;
            }
            self.function_map
                .get(&function.old_addr)
                .and_then(|list| list.last())
                .filter(|patch_function| !patch_function.is_same_function(uuid, function))
        })
    }
}
