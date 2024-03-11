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

use std::{env, path::Path, process::Command};

fn rewrite_version() {
    const ENV_VERSION_NAME: &str = "BUILD_VERSION";
    const PKG_VERSION_NAME: &str = "CARGO_PKG_VERSION";

    let version = env::var(ENV_VERSION_NAME).unwrap_or_else(|_| {
        let pkg_version = env::var(PKG_VERSION_NAME).expect("Failed to fetch package version");
        let git_output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .map(|output| String::from_utf8(output.stdout).expect("Failed to fetch git version"));

        match git_output {
            Ok(git_version) => format!("{}-g{}", pkg_version, git_version),
            Err(_) => pkg_version,
        }
    });

    println!("cargo:rustc-env={}={}", PKG_VERSION_NAME, version);
}

fn build_ffi_library() {
    const UPATCH_LIB: &str = "../upatch-lib";

    cc::Build::new()
        .files(&[
            Path::new(UPATCH_LIB).join("upatch-common.c"),
            Path::new(UPATCH_LIB).join("upatch-elf.c"),
            Path::new(UPATCH_LIB).join("upatch-ioctl.c"),
            Path::new(UPATCH_LIB).join("upatch-meta.c"),
            Path::new(UPATCH_LIB).join("upatch-resolve.c"),
            Path::new(UPATCH_LIB).join("upatch-tool-lib.c"),
        ])
        .includes(&[UPATCH_LIB])
        .compile("libupatch.a");
}

fn main() {
    rewrite_version();
    build_ffi_library();
}
