// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatch-helper is licensed under Mulan PSL v2.
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
    os::unix::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use uuid::Uuid;

const COMPILER_KEYWORDS_CC: &[&str] = &["cc", "clang"];
const COMPILER_KEYWORDS_CXX: &[&str] = &["++", "xx"];

const COMPILER_EXCLUDE_FLAGS: &[&str] = &[
    "-E",            // Preprocess only; does not compile.
    "--version",     // Print compiler version and exit.
    "--help",        // Print help message and exit.
    "--target-help", // Print target-specific help and exit.
    "-dumpversion",  // Print compiler version string (e.g., "11.2.0") and exit.
    "-dumpmachine",  // Print compiler target machine (e.g., "x86_64-linux-gnu") and exit.
    "-###",          // Dry run: print commands that would be executed, but do not run them.
];
const COMPILER_EXCLUDE_FLAG_PREFIXES: &[&str] = &["--print-"];
const COMPILER_COMPILE_SIGNAL_FLAGS: &[&str] = &["-x"];
const COMPILER_SPECIAL_SOURCE_FILES: &[&str] = &["-", "@args.txt"];
const COMPILER_SOURCE_FILE_EXTENSIONS: &[&str] = &["c", "cc", "cpp", "cxx", "s", "S"];

const HELPER_ENV_NAME_CC: &str = "UPATCH_HELPER_CC";
const HELPER_ENV_NAME_CXX: &str = "UPATCH_HELPER_CXX";
const HELPER_ENV_NAMES: &[(&[&str], &str)] = &[
    (COMPILER_KEYWORDS_CC, HELPER_ENV_NAME_CC),
    (COMPILER_KEYWORDS_CXX, HELPER_ENV_NAME_CXX),
];

const COMPILE_OPTIONS_GNU: &[&str] = &[
    "-gdwarf",                 // generate dwarf debuginfo
    "-ffunction-sections",     // generate corresponding section for each function
    "-fdata-sections",         // generate corresponding section for each data
    "-fmerge-constants",       // merge constants with same value into one
    "-fno-common",             // avoid generating common block for uninitialized global variables
    "-fno-tree-slp-vectorize", // avoid converting scalar operations into SIMD instructions
];
const COMPILE_OPTIONS_CLANG: &[&str] = &[
    "-gdwarf",
    "-ffunction-sections",
    "-fdata-sections",
    "-fno-common",
    "-fno-slp-vectorize", // avoid converting scalar operations into SIMD instructions
    "-fno-integrated-as", // avoid using built-in llvm-as, which doesn't support "--defsym"
    "-Werror=uninitialized", // uninitialized variable may generate unexpected code under O2 optimization
];

const UPATCH_ID_PREFIX: &str = ".upatch_";

#[inline(always)]
fn is_compilation(args: &[OsString]) -> bool {
    /* check exclude flags */
    for arg in args.iter().skip(1) {
        if COMPILER_EXCLUDE_FLAGS
            .iter()
            .any(|&flag| arg == OsStr::new(flag))
        {
            return false;
        }
        if COMPILER_EXCLUDE_FLAG_PREFIXES
            .iter()
            .any(|&prefix| arg.as_bytes().starts_with(prefix.as_bytes()))
        {
            return false;
        }
    }

    /* check compile flag & source file */
    for arg in args.iter().skip(1) {
        if COMPILER_COMPILE_SIGNAL_FLAGS
            .iter()
            .any(|&name| arg == OsStr::new(name))
        {
            return true;
        }
        if COMPILER_SPECIAL_SOURCE_FILES
            .iter()
            .any(|&name| arg == OsStr::new(name))
        {
            return true;
        }
        if let Some(src_ext) = Path::new(arg).extension() {
            if COMPILER_SOURCE_FILE_EXTENSIONS
                .iter()
                .any(|&ext| src_ext == OsStr::new(ext))
            {
                return true;
            }
        }
    }

    false
}

#[inline(always)]
fn find_compiler(arg0: &OsStr) -> Result<PathBuf> {
    let file_name = Path::new(arg0).file_name().unwrap_or_default();

    // match compiler by file name
    let env_entry = HELPER_ENV_NAMES.iter().find(|(keys, _)| {
        keys.iter().any(|str| {
            let key_bytes = str.as_bytes();
            file_name
                .as_bytes()
                .windows(key_bytes.len())
                .any(|window| window == key_bytes)
        })
    });
    if let Some((_, env_name)) = env_entry {
        return std::env::var_os(env_name)
            .map(PathBuf::from)
            .with_context(|| format!("Environment variable '{}' was not set", env_name));
    }

    // exec name matched, read environment variable directly
    let exec_path = std::env::current_exe()?;
    let exec_name = exec_path.file_name().unwrap_or_default();
    if exec_name == file_name {
        return HELPER_ENV_NAMES
            .iter()
            .rev()
            .find_map(|&(_, env_name)| std::env::var_os(env_name).map(PathBuf::from))
            .with_context(|| {
                format!(
                    "Environment variables '{}' and '{}' were not set",
                    HELPER_ENV_NAME_CC, HELPER_ENV_NAME_CXX
                )
            });
    }

    bail!("No compiler found");
}

#[inline(always)]
fn add_compile_options(command: &mut Command) {
    let prog_name = Path::new(command.get_program())
        .file_name()
        .unwrap_or_default();
    let clang_name_bytes = COMPILER_KEYWORDS_CC[1].as_bytes();
    let is_clang = prog_name
        .as_bytes()
        .windows(clang_name_bytes.len())
        .any(|window| window == clang_name_bytes);

    let compiler_args = match is_clang {
        true => COMPILE_OPTIONS_CLANG,
        false => COMPILE_OPTIONS_GNU,
    };
    let assembler_arg = format!("-Wa,--defsym,{}{}=0", UPATCH_ID_PREFIX, Uuid::new_v4());

    command.args(compiler_args);
    command.arg(assembler_arg);
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let compiler = self::find_compiler(&args[0])?;

    let mut command = Command::new(&compiler);
    command.args(&args[1..]);
    if self::is_compilation(&args) {
        self::add_compile_options(&mut command);
    }

    let err = command.exec();
    bail!(
        "Failed to execute '{}', {}",
        compiler.display(),
        err.to_string().to_lowercase()
    );
}
