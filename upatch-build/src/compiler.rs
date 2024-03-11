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
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use log::debug;
use which::which;

use syscare_common::{ffi::OsStrExt, fs, process::Command};

use super::dwarf::Dwarf;

const ASSEMBLER_NAME: &str = "as";
const LINKER_NAME: &str = "ld";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Compiler {
    pub name: OsString,
    pub path: PathBuf,
    pub assembler: PathBuf,
    pub linker: PathBuf,
}

impl Compiler {
    fn get_component_name<P: AsRef<Path>>(compiler_path: P, name: &str) -> Result<OsString> {
        let output = Command::new(compiler_path.as_ref())
            .arg(format!("-print-prog-name={}", name))
            .run_with_output()?;

        output.exit_ok()?;
        Ok(output.stdout)
    }

    fn run_test_build<P, Q>(compiler_path: P, temp_dir: Q) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let temp_dir = temp_dir.as_ref();
        let source_file = temp_dir.join("test.c");
        let output_file = temp_dir.join("test.o");

        fs::write(&source_file, "int main() { return 0; }")
            .context("Failed to write source file")?;

        Command::new(compiler_path.as_ref())
            .args(["-gdwarf", "-ffunction-sections", "-fdata-sections", "-c"])
            .arg(&source_file)
            .arg("-o")
            .arg(&output_file)
            .run()?
            .exit_ok()?;

        Ok(output_file)
    }

    fn get_compiler_name<P, Q>(compiler_path: P, temp_dir: Q) -> Result<OsString>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let test_object =
            Self::run_test_build(compiler_path, temp_dir).context("Compiler test build failed")?;

        Dwarf::parse_compiler_name(&test_object)
            .with_context(|| format!("Failed to parse compiler name of {}", test_object.display()))
    }
}

impl Compiler {
    pub fn parse<I, P, Q>(compilers: I, temp_dir: Q) -> Result<Vec<Compiler>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let mut compiler_list = Vec::new();

        for compiler in compilers {
            let compiler_path = compiler.as_ref();
            let compiler_name = compiler_path
                .file_name()
                .context("Failed to parse compiler name")?;
            let temp_dir = temp_dir.as_ref().join(compiler_name);
            fs::create_dir_all(&temp_dir)?;

            debug!("- Checking {}", compiler_path.display());
            let assembler_name = Self::get_component_name(compiler_path, ASSEMBLER_NAME)
                .with_context(|| {
                    format!(
                        "Failed to get assembler name of {}",
                        compiler_path.display()
                    )
                })?;
            let linker_name =
                Self::get_component_name(compiler_path, LINKER_NAME).with_context(|| {
                    format!("Failed to get linker name of {}", compiler_path.display())
                })?;

            let name = Self::get_compiler_name(compiler_path, temp_dir).with_context(|| {
                format!("Failed to get compiler name of {}", compiler_path.display())
            })?;
            let path = compiler_path.to_path_buf();
            let assembler = which(assembler_name.trim()).with_context(|| {
                format!("Cannot find assembler {}", assembler_name.to_string_lossy())
            })?;
            let linker = which(linker_name.trim())
                .with_context(|| format!("Cannot find linker {}", linker_name.to_string_lossy()))?;

            compiler_list.push(Compiler {
                name,
                path,
                assembler,
                linker,
            });
        }

        compiler_list.sort();
        Ok(compiler_list)
    }

    pub fn link_objects<I, S, P>(&self, objects: I, output: P) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<Path>,
    {
        Command::new(&self.linker)
            .args(["-r", "-o"])
            .arg(output.as_ref())
            .args(objects)
            .run()?
            .exit_ok()
    }
}

impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {}, path: {}, assembler: {}, linker: {}",
            self.name.to_string_lossy(),
            self.path.display(),
            self.assembler.display(),
            self.linker.display(),
        )
    }
}
