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

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Result;
use indexmap::IndexSet;
use syscare_common::fs;

use syscare_common::os::proc_maps::ProcMaps;

#[inline]
fn parse_process_id<P: AsRef<Path>>(path: P) -> Option<i32> {
    path.as_ref()
        .file_name()
        .and_then(OsStr::to_str)
        .map(str::parse)
        .and_then(Result::ok)
}

#[inline]
fn parse_process_path(pid: i32) -> Option<(i32, PathBuf)> {
    const PROC_BLACK_LIST: [&str; 18] = [
        "/usr/lib/systemd/systemd-journald",
        "/usr/lib/systemd/systemd-logind",
        "/usr/lib/systemd/systemd-udevd",
        "/usr/lib/systemd/systemd-hostnamed",
        "/usr/bin/udevadm",
        "/usr/sbin/auditd",
        "/usr/bin/syscare",
        "/usr/bin/syscared",
        "/usr/bin/upatchd",
        "/usr/libexec/syscare/as-hijacker",
        "/usr/libexec/syscare/cc-hijacker",
        "/usr/libexec/syscare/c++-hijacker",
        "/usr/libexec/syscare/gcc-hijacker",
        "/usr/libexec/syscare/g++-hijacker",
        "/usr/libexec/syscare/syscare-build",
        "/usr/libexec/syscare/upatch-build",
        "/usr/libexec/syscare/upatch-diff",
        "/usr/libexec/syscare/upatch-manage",
    ];

    fs::read_link(format!("/proc/{}/exe", pid))
        .ok()
        .filter(|path| {
            !PROC_BLACK_LIST
                .iter()
                .any(|blacklist_path| path.as_os_str() == *blacklist_path)
        })
        .map(|path| (pid, path))
}

pub fn find_target_process<P: AsRef<Path>>(target_elf: P) -> Result<IndexSet<i32>> {
    let target_file = fs::canonicalize(target_elf.as_ref())?;
    let target_path = target_file.as_path();
    let target_pids = fs::list_dirs("/proc", fs::TraverseOptions { recursive: false })?
        .into_iter()
        .filter_map(self::parse_process_id)
        .filter_map(self::parse_process_path)
        .filter(|(pid, bin_path)| {
            if bin_path == target_path {
                return true;
            }
            if let Ok(mut mappings) = ProcMaps::new(*pid) {
                return mappings.any(|map| map.path_name == target_path);
            }
            false
        })
        .map(|(pid, _)| pid)
        .collect();

    Ok(target_pids)
}
