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
use indexmap::IndexSet;
use log::debug;
use which::which;

use syscare_common::{ffi::OsStrExt, fs, process::Command};

use super::dwarf::Dwarf;

const COMPILER_STANDARDS: [&str; 42] = [
    "c89",
    "c90",
    "iso9899:1990",
    "iso9899:199409",
    "c99",
    "c9x",
    "iso9899:1999",
    "iso9899:199x",
    "c11",
    "c1x",
    "iso9899:2011",
    "c17",
    "c18",
    "iso9899:2017",
    "iso9899:2018",
    "gnu89",
    "gnu90",
    "gnu99",
    "gnu9x",
    "gnu11",
    "gnu1x",
    "gnu17",
    "gnu18",
    "gnu2x",
    "c++98",
    "c++03",
    "c++11",
    "c++0x",
    "gnu++11",
    "gnu++0x",
    "c++14",
    "c++1y",
    "gnu++14",
    "gnu++1y",
    "c++17",
    "c++1z",
    "gnu++17",
    "gnu++1z",
    "c++20",
    "c++2a",
    "gnu++20",
    "gnu++2a",
];
const ASSEMBLER_NAME: &str = "as";
const LINKER_NAME: &str = "ld";

#[derive(Debug, Clone)]
pub struct Compiler {
    pub path: PathBuf,
    pub assembler: PathBuf,
    pub linker: PathBuf,
    pub versions: IndexSet<OsString>,
}

impl Compiler {
    fn get_component_name<P: AsRef<Path>>(compiler: P, name: &str) -> Result<OsString> {
        let output = Command::new(compiler.as_ref())
            .arg(format!("-print-prog-name={}", name))
            .run_with_output()?;

        output.exit_ok()?;
        Ok(output.stdout)
    }

    fn run_assembler_test<P, Q>(&self, source_file: P, output_dir: Q) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let assembler_file = output_dir.as_ref().join("test.s");
        let object_file = output_dir.as_ref().join("test.o");

        Command::new(&self.path)
            .arg("-S")
            .arg(source_file.as_ref())
            .arg("-o")
            .arg(assembler_file.as_path())
            .run()?
            .exit_ok()?;

        Command::new(&self.assembler)
            .arg("-g")
            .arg(assembler_file)
            .arg("-o")
            .arg(object_file.as_path())
            .run()?
            .exit_ok()?;

        Ok(object_file)
    }

    fn run_compiler_test<P, Q>(&self, source_file: P, output_dir: Q) -> Result<Vec<PathBuf>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut object_files = Vec::new();

        for std_name in COMPILER_STANDARDS {
            let object_file = output_dir.as_ref().join(format!("test_{}.o", std_name));
            let build_success = Command::new(&self.path)
                .arg(format!("-std={}", std_name))
                .args(["-g", "-c"])
                .arg(source_file.as_ref())
                .arg("-o")
                .arg(object_file.as_path())
                .run()?
                .success();

            if build_success {
                object_files.push(object_file);
            }
        }

        Ok(object_files)
    }

    fn fetch_versions<P: AsRef<Path>>(&mut self, output_dir: P) -> Result<()> {
        let source_file = output_dir.as_ref().join("test.c");
        fs::write(&source_file, "int main() { return 0; }")
            .context("Failed to write source file")?;

        let mut objects = self
            .run_compiler_test(&source_file, &output_dir)
            .context("Compiler test failed")?;
        let asm_object = self
            .run_assembler_test(&source_file, &output_dir)
            .context("Assembler test failed")?;
        objects.push(asm_object);

        for object in objects {
            let versions = Dwarf::parse_compiler_versions(&object).with_context(|| {
                format!("Failed to parse compiler name of {}", object.display())
            })?;
            self.versions.extend(versions);
        }

        Ok(())
    }
}

impl Compiler {
    pub fn parse<I, P, Q>(compilers: I, temp_dir: Q) -> Result<Vec<Compiler>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut result = Vec::new();

        for compiler in compilers {
            let compiler = compiler.as_ref();
            let compiler_name = compiler
                .file_name()
                .context("Failed to parse compiler name")?;

            let output_dir = temp_dir.as_ref().join(compiler_name);
            fs::create_dir_all(&output_dir)?;

            debug!("- Checking {}", compiler.display());
            let assembler_name =
                Self::get_component_name(compiler, ASSEMBLER_NAME).with_context(|| {
                    format!("Failed to get assembler name of {}", compiler.display())
                })?;
            let linker_name = Self::get_component_name(compiler, LINKER_NAME)
                .with_context(|| format!("Failed to get linker name of {}", compiler.display()))?;

            let path = compiler.to_path_buf();
            let assembler = which(assembler_name.trim()).with_context(|| {
                format!("Cannot find assembler {}", assembler_name.to_string_lossy())
            })?;
            let linker = which(linker_name.trim())
                .with_context(|| format!("Cannot find linker {}", linker_name.to_string_lossy()))?;
            let versions = IndexSet::new();

            let mut compiler = Self {
                path,
                assembler,
                linker,
                versions,
            };
            compiler
                .fetch_versions(output_dir)
                .context("Failed to fetch supported versions")?;

            result.push(compiler);
        }

        Ok(result)
    }
}

impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path: {}, assembler: {}, linker: {}",
            self.path.display(),
            self.assembler.display(),
            self.linker.display()
        )
    }
}
