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
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use indexmap::{IndexMap, IndexSet};
use log::debug;
use which::which;

use syscare_common::{ffi::OsStrExt, fs, process::Command};

use crate::dwarf::{Dwarf, ProducerType};

const STD_NAMES: [&str; 45] = [
    "c89", "c90", "c99", "c9x", "c11", "c17", "c18", "c1x", "c2x", "gnu", "gnu89", "gnu90",
    "gnu99", "gnu9x", "gnu11", "gnu17", "gnu18", "gnu1x", "gnu2x", "c++98", "c++03", "c++0x",
    "c++11", "c++14", "c++17", "c++1y", "c++1z", "c++20", "c++2a", "gnu++98", "gnu++03", "gnu++0x",
    "gnu++11", "gnu++14", "gnu++17", "gnu++1y", "gnu++1z", "gnu++20", "gnu++2a", "f95", "f2003",
    "f2008", "f2008ts", "f2018", "legacy",
];
const ASSEMBLER_NAME: &str = "as";
const LINKER_NAME: &str = "ld";

#[derive(Debug, Clone)]
pub struct CompilerInfo {
    pub binary: PathBuf,
    pub assembler: PathBuf,
    pub linker: PathBuf,
    pub producers: IndexSet<OsString>,
}

impl CompilerInfo {
    fn get_component_name<P: AsRef<Path>>(compiler: P, name: &str) -> Result<OsString> {
        let output = Command::new(compiler.as_ref())
            .arg(format!("-print-prog-name={}", name))
            .run_with_output()?;

        output.exit_ok()?;
        Ok(output.stdout)
    }

    fn build_test_objects<P, Q, R>(
        binary: P,
        assembler: Q,
        output_dir: R,
    ) -> Result<IndexSet<PathBuf>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
    {
        let source_file = output_dir.as_ref().join("test.c");
        let assembly_file = output_dir.as_ref().join("test.S");
        let assembly_object = output_dir.as_ref().join("test.o");

        fs::write(&source_file, "int main() { return 0; }")?;

        let mut object_files = IndexSet::new();

        Command::new(binary.as_ref())
            .arg("-S")
            .arg(&source_file)
            .arg("-o")
            .arg(&assembly_file)
            .run()?
            .exit_ok()?;

        Command::new(assembler.as_ref())
            .arg("-g")
            .arg(assembly_file)
            .arg("-o")
            .arg(&assembly_object)
            .run()?
            .exit_ok()?;

        object_files.insert(assembly_object);

        for std_name in STD_NAMES {
            let compiler_object = output_dir.as_ref().join(format!("test_{}.o", std_name));
            let build_success = Command::new(binary.as_ref())
                .arg(format!("-std={}", std_name))
                .args(["-g", "-c"])
                .arg(&source_file)
                .arg("-o")
                .arg(&compiler_object)
                .stderr(log::Level::Trace)
                .run_with_output()?
                .success();

            if build_success {
                object_files.insert(compiler_object);
            }
        }
        object_files.sort();

        Ok(object_files)
    }

    fn run_compiler_detection<P, Q, R>(
        binary: P,
        assembler: Q,
        output_dir: R,
    ) -> Result<IndexSet<OsString>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
    {
        let mut producers = IndexSet::new();

        let test_objects = Self::build_test_objects(binary, assembler, &output_dir)
            .context("Failed to build test objects")?;
        for test_object in test_objects {
            producers.extend(Dwarf::parse(test_object)?.producers());
        }
        producers.sort();

        fs::remove_dir_all(&output_dir)?;
        Ok(producers)
    }
}

impl CompilerInfo {
    pub fn parse<I, P, Q>(compilers: I, temp_dir: Q) -> Result<IndexMap<ProducerType, CompilerInfo>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut compiler_map = IndexMap::new();

        for compiler in compilers {
            let binary_file = compiler.as_ref();
            let binary_name = binary_file
                .file_name()
                .context("Failed to parse binary name")?;

            let output_dir = temp_dir.as_ref().join(binary_name);
            fs::create_dir_all(&output_dir)?;

            debug!("- Checking {}", binary_file.display());
            let assembler_name = Self::get_component_name(binary_file, ASSEMBLER_NAME)
                .with_context(|| {
                    format!("Failed to get assembler name of {}", binary_file.display())
                })?;
            let assembler = fs::normalize(which(assembler_name.trim()).with_context(|| {
                format!("Cannot find assembler {}", assembler_name.to_string_lossy())
            })?)?;

            let linker_name =
                Self::get_component_name(binary_file, LINKER_NAME).with_context(|| {
                    format!("Failed to get linker name of {}", binary_file.display())
                })?;
            let linker = fs::normalize(which(linker_name.trim()).with_context(|| {
                format!("Cannot find linker {}", linker_name.to_string_lossy())
            })?)?;
            let producers = Self::run_compiler_detection(&compiler, &assembler, &output_dir)
                .context("Failed to detect compiler")?;

            for producer in &producers {
                let producer_type = ProducerType::from(producer);
                if (producer_type == ProducerType::As) || (producer_type == ProducerType::Unknown) {
                    continue;
                }
                compiler_map.insert(
                    producer_type,
                    Self {
                        binary: binary_file.to_path_buf(),
                        assembler: assembler.clone(),
                        linker: linker.clone(),
                        producers: producers.clone(),
                    },
                );
            }
        }
        compiler_map.sort_keys();

        Ok(compiler_map)
    }
}
