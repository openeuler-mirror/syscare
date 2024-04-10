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

use std::{ffi::OsStr, os::linux::fs::MetadataExt, path::Path};

use anyhow::Result;
use indexmap::IndexSet;
use syscare_common::fs;

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

#[inline]
fn is_blacklisted(file_path: &Path) -> bool {
    PROC_BLACK_LIST
        .iter()
        .map(Path::new)
        .any(|blacklist_path| blacklist_path == file_path)
}

#[inline]
fn parse_process_id(proc_path: &Path) -> Option<i32> {
    proc_path
        .file_name()
        .and_then(OsStr::to_str)
        .map(str::parse)
        .and_then(Result::ok)
}

pub fn find_target_process<P: AsRef<Path>>(target_elf: P) -> Result<IndexSet<i32>> {
    let mut target_pids = IndexSet::new();
    let target_path = target_elf.as_ref();
    let target_inode = target_path.metadata()?.st_ino();

    for proc_path in fs::list_dirs("/proc", fs::TraverseOptions { recursive: false })? {
        let pid = match self::parse_process_id(&proc_path) {
            Some(pid) => pid,
            None => continue,
        };
        let exec_path = match fs::read_link(format!("/proc/{}/exe", pid)) {
            Ok(file_path) => file_path,
            Err(_) => continue,
        };
        if is_blacklisted(&exec_path) {
            continue;
        }
        // Try to match binary path
        if exec_path == target_path {
            target_pids.insert(pid);
            continue;
        }
        // Try to match mapped files
        let map_files = fs::list_symlinks(
            format!("/proc/{}/map_files", pid),
            fs::TraverseOptions { recursive: false },
        )?;
        for mapped_file in map_files {
            if let Ok(mapped_inode) = mapped_file
                .read_link()
                .and_then(|file_path| Ok(file_path.metadata()?.st_ino()))
            {
                if mapped_inode == target_inode {
                    target_pids.insert(pid);
                    break;
                }
            };
        }
    }

    Ok(target_pids)
}
