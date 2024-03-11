// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{fs, path::PathBuf};

use anyhow::Result;
use object::{NativeFile, Object, ObjectSymbol};
use syscare_common::{ffi::OsStrExt, os, process::Command};

const LDD_BIN: &str = "ldd";

pub struct ElfResolver<'a> {
    elf: NativeFile<'a, &'a [u8]>,
}

impl<'a> ElfResolver<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        Ok(Self {
            elf: NativeFile::parse(data)?,
        })
    }
}

impl ElfResolver<'_> {
    pub fn dependencies(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let output = Command::new(LDD_BIN)
            .arg(os::process::path())
            .run_with_output()?;

        output.exit_ok()?;

        let lines = output.stdout.lines().filter_map(|s| s.ok());
        for line in lines {
            let words = line.split_whitespace().collect::<Vec<_>>();
            if let Some(path) = words.get(2) {
                if let Ok(path) = fs::canonicalize(path) {
                    paths.push(path);
                }
            }
        }

        Ok(paths)
    }

    pub fn find_symbol_addr(&self, symbol_name: &str) -> Result<Option<u64>> {
        let symbols = self.elf.dynamic_symbols();
        for sym in symbols {
            if let Ok(sym_name) = sym.name() {
                if sym_name == symbol_name {
                    return Ok(Some(sym.address()));
                }
            }
        }

        Ok(None)
    }
}
