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

use std::ffi::{OsStr, OsString};

use indexmap::IndexMap;
use uuid::Uuid;

use crate::patch::entity::KernelPatchSymbol;

#[derive(Debug, PartialEq)]
pub struct PatchTargetRecord {
    pub uuid: Uuid,
    pub name: OsString,
    pub size: u64,
}

pub struct PatchTarget {
    name: OsString,
    symbol_map: IndexMap<u64, Vec<PatchTargetRecord>>, // symbol addr -> symbol collision list
}

impl PatchTarget {
    fn match_record(record: &PatchTargetRecord, uuid: &Uuid, symbol: &KernelPatchSymbol) -> bool {
        (record.uuid == *uuid) && (record.name == symbol.name) && (record.size == symbol.new_size)
    }
}

impl PatchTarget {
    pub fn new<S: AsRef<OsStr>>(name: S) -> Self {
        Self {
            name: name.as_ref().to_os_string(),
            symbol_map: IndexMap::new(),
        }
    }

    pub fn classify_symbols(
        symbols: &[KernelPatchSymbol],
    ) -> IndexMap<&OsStr, Vec<&KernelPatchSymbol>> {
        let mut symbol_map = IndexMap::new();

        for symbol in symbols {
            let target_name = symbol.target.as_os_str();

            symbol_map
                .entry(target_name)
                .or_insert_with(Vec::new)
                .push(symbol);
        }

        symbol_map
    }

    pub fn get_conflicts<'a, I>(
        &'a self,
        symbols: I,
    ) -> impl IntoIterator<Item = &'a PatchTargetRecord>
    where
        I: IntoIterator<Item = &'a KernelPatchSymbol>,
    {
        symbols.into_iter().filter_map(move |symbol| {
            if self.name != symbol.target {
                return None;
            }
            self.symbol_map
                .get(&symbol.old_addr)
                .and_then(|list| list.last())
        })
    }

    pub fn get_overrides<'a, I>(
        &'a self,
        uuid: &'a Uuid,
        symbols: I,
    ) -> impl IntoIterator<Item = &'a PatchTargetRecord>
    where
        I: IntoIterator<Item = &'a KernelPatchSymbol>,
    {
        symbols.into_iter().filter_map(move |symbol| {
            if self.name != symbol.target {
                return None;
            }
            self.symbol_map
                .get(&symbol.old_addr)
                .and_then(|list| list.last())
                .filter(|record| !Self::match_record(record, uuid, symbol))
        })
    }

    pub fn add_symbols<'a, I>(&mut self, uuid: Uuid, symbols: I)
    where
        I: IntoIterator<Item = &'a KernelPatchSymbol>,
    {
        for symbol in symbols {
            if self.name != symbol.target {
                continue;
            }

            let symbol_addr = symbol.old_addr;
            let symbol_record = PatchTargetRecord {
                uuid,
                name: symbol.name.to_os_string(),
                size: symbol.new_size,
            };

            self.symbol_map
                .entry(symbol_addr)
                .or_default()
                .push(symbol_record);
        }
    }

    pub fn remove_symbols<'a, I>(&mut self, uuid: &Uuid, symbols: I)
    where
        I: IntoIterator<Item = &'a KernelPatchSymbol>,
    {
        for symbol in symbols {
            if self.name != symbol.target {
                continue;
            }

            let symbol_addr = symbol.old_addr;
            if let Some(collision_list) = self.symbol_map.get_mut(&symbol_addr) {
                if let Some(index) = collision_list
                    .iter()
                    .position(|record| Self::match_record(record, uuid, symbol))
                {
                    collision_list.remove(index);
                    if collision_list.is_empty() {
                        self.symbol_map.remove(&symbol_addr);
                    }
                }
            }
        }
    }
}
