// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-build is licensed under Mulan PSL v2.
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
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use indexmap::IndexSet;
use which::which;

use syscare_common::{concat_os, ffi::OsStrExt as _, fs, process::Command};

use crate::dwarf::{Producer, ProducerParser, ProducerType};

#[derive(Debug, Clone)]
pub struct Compiler {
    pub prefix: Option<OsString>,
    pub name: OsString,
    pub kind: ProducerType,
    pub version: OsString,
    pub path: PathBuf,
    pub linker: PathBuf,
}

impl Compiler {
    pub fn parse<P, Q>(path: P, output_dir: Q) -> Result<Compiler>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let path = path.as_ref().to_path_buf();
        let name = path
            .file_name()
            .context("Failed to parse compiler name")?
            .to_os_string();

        let output_path = Self::build_test_object(&path, &name, output_dir.as_ref())
            .context("Failed to build test object")?;
        let prefix = Self::parse_compiler_prefix(&name).map(OsStr::to_os_string);
        let producer = Self::parse_compiler_producer(&output_path)
            .context("Failed to parse compiler producer")?;
        let linker = Self::get_compiler_linker(&path, &prefix, &producer)
            .context("Failed to get compiler linker")?;

        Ok(Self {
            prefix,
            name,
            kind: producer.kind,
            version: producer.version,
            path,
            linker,
        })
    }

    fn build_test_object(path: &Path, name: &OsStr, output_dir: &Path) -> Result<PathBuf> {
        let source_file = output_dir.join("test.c");
        let output_file = output_dir.join(concat_os!(name, "-test"));

        if !source_file.exists() {
            fs::write(&source_file, "int main() { return 0; }")?;
        }
        Command::new(path)
            .arg("-g")
            .arg(&source_file)
            .arg("-o")
            .arg(&output_file)
            .run()?
            .exit_ok()?;

        Ok(output_file)
    }

    fn parse_compiler_prefix(compiler: &OsStr) -> Option<&OsStr> {
        /*
         * Matches compiler prefix of compiler name
         * eg. x86_64-linux-gnu-gcc       -> x86_64-linux-gnu-
         * eg. aarch64-target-linux-clang -> aarch64-target-linux-
         */
        let slice = compiler.as_bytes();

        let spliter_indices = slice.iter().enumerate().rev().filter_map(|(index, &b)| {
            if b == b'-' {
                return Some(index);
            }
            None
        });

        for pos in spliter_indices {
            let (prefix, name) = slice.split_at(pos + 1);
            if name.iter().any(|&b| !b.is_ascii_digit()) {
                return Some(OsStr::from_bytes(prefix));
            }
        }

        None
    }

    fn parse_compiler_producer(path: &Path) -> Result<Producer> {
        let producer_parser = ProducerParser::open(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        let producer_iter = producer_parser
            .parse()
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        let mut producer_map = IndexSet::new();
        for parse_result in producer_iter {
            let producer = parse_result.context("Failed to parse object producer")?;
            producer_map.insert(producer);
        }
        producer_map.sort();

        // Compiler producer would be highest supported.
        producer_map.pop().context("No object producer")
    }

    fn get_component(
        path: &Path,
        prefix: &Option<OsString>,
        name: &str,
    ) -> Result<Option<PathBuf>> {
        let get_component_path = |name: &OsStr| -> Result<Option<PathBuf>> {
            let output = Command::new(path)
                .arg(concat_os!("-print-prog-name=", name))
                .run_with_output()?;
            output.exit_ok()?;
            Ok(which(output.stdout.trim()).ok())
        };

        if let Some(prefixed_name) = prefix.as_ref().map(|pfx| concat_os!(pfx, name)) {
            let component = get_component_path(&prefixed_name)?;
            if component.is_some() {
                return Ok(component);
            }
        }
        get_component_path(OsStr::new(name))
    }

    fn get_compiler_linker(
        path: &Path,
        prefix: &Option<OsString>,
        producer: &Producer,
    ) -> Result<PathBuf> {
        const CLANG_LINKER_NAMES: &[&str] = &["ld.lld", "ld"];
        const GNU_LINKER_NAMES: &[&str] = &["ld"];

        let linkers = if matches!(producer.kind, ProducerType::ClangC | ProducerType::ClangCxx) {
            // Clang may use llvm linker, we will try it firstly.
            CLANG_LINKER_NAMES
        } else {
            GNU_LINKER_NAMES
        };
        for name in linkers {
            if let Some(path) = Self::get_component(path, prefix, name)? {
                return Ok(path);
            }
        }
        bail!("No suitable linker")
    }
}

impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}{}",
            self.prefix.as_deref().unwrap_or_default().to_string_lossy(),
            self.name.to_string_lossy()
        ))
    }
}
