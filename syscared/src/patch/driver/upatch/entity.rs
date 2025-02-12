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

use std::path::PathBuf;

use indexmap::{indexset, IndexSet};

#[derive(Debug)]
pub struct PatchEntity {
    pub patch_file: PathBuf,
    process_list: IndexSet<i32>,
}

impl PatchEntity {
    pub fn new(patch_file: PathBuf) -> Self {
        Self {
            patch_file,
            process_list: indexset! {},
        }
    }
}

impl PatchEntity {
    pub fn add_process(&mut self, pid: i32) {
        self.process_list.insert(pid);
    }

    pub fn remove_process(&mut self, pid: i32) {
        self.process_list.remove(&pid);
    }

    pub fn clean_dead_process(&mut self, process_list: &IndexSet<i32>) {
        self.process_list.retain(|pid| process_list.contains(pid));
    }

    pub fn need_actived(&self, process_list: &IndexSet<i32>) -> IndexSet<i32> {
        process_list
            .difference(&self.process_list)
            .copied()
            .collect()
    }

    pub fn need_deactived(&self, process_list: &IndexSet<i32>) -> IndexSet<i32> {
        process_list
            .intersection(&self.process_list)
            .copied()
            .collect()
    }
}
