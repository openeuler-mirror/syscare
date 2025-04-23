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

use std::{os::unix::process::CommandExt, path::Path, process::Command};

use anyhow::{bail, Context};
use uuid::Uuid;

const UPATCH_CC_ENV: &str = "UPATCH_HELPER_CC";
const UPATCH_CXX_ENV: &str = "UPATCH_HELPER_CXX";
const UPATCH_ID_PREFIX: &str = ".upatch_";

const OUTPUT_FLAG: &str = "-o";

const APPEND_FLAGS: &[&str] = &[
    "-gdwarf",             // generate dwarf debuginfo
    "-ffunction-sections", // generate corresponding section for each function
    "-fdata-sections",     // generate corresponding section for each data
    "-fmerge-constants",   // merge constants with same value into one
    "-fno-common",         // avoid generating common block for uninitialized global variables
];

fn main() -> anyhow::Result<()> {
    let exec_args = std::env::args_os().collect::<Vec<_>>();
    let exec_name = Path::new(&exec_args[0])
        .file_name()
        .context("Cannot parse exec name")?
        .to_string_lossy();
    let exec_path = if exec_name.contains("cc") {
        std::env::var_os(UPATCH_CC_ENV)
            .with_context(|| format!("Environment variable '{}' is not set", UPATCH_CC_ENV))?
    } else if exec_name.contains("++") {
        std::env::var_os(UPATCH_CXX_ENV)
            .with_context(|| format!("Environment variable '{}' is not set", UPATCH_CXX_ENV))?
    } else {
        bail!("Invalid exec name '{}'", exec_name);
    };

    let mut command = Command::new(&exec_path);

    command.args(exec_args.iter().skip(1));
    if exec_args.iter().any(|arg| arg == OUTPUT_FLAG) {
        command.args(APPEND_FLAGS);
        command.arg(format!(
            "-Wa,--defsym,{}{}=0",
            UPATCH_ID_PREFIX,
            Uuid::new_v4(),
        ));
    }

    let err = command.exec();
    bail!(
        "Exec '{}' failed, {}",
        exec_path.to_string_lossy(),
        err.to_string().to_lowercase()
    );
}
