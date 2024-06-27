// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-build is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{env, ffi::OsStr, os::unix::ffi::OsStrExt, process::Command};

fn rewrite_version() {
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const ENV_VERSION: Option<&str> = option_env!("BUILD_VERSION");

    println!(
        "cargo:rustc-env=CARGO_PKG_VERSION={}",
        ENV_VERSION.map(String::from).unwrap_or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .map(|output| {
                    let git_version = OsStr::from_bytes(&output.stdout).to_string_lossy();
                    format!("{}-g{}", PKG_VERSION, git_version)
                })
                .unwrap_or_else(|_| PKG_VERSION.to_string())
        })
    );
}

fn main() {
    rewrite_version();
}
