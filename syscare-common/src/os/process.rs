// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-common is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use nix::unistd::getpid;

use crate::fs;

pub fn id() -> i32 {
    lazy_static! {
        static ref PROCESS_ID: i32 = getpid().as_raw();
    }
    *PROCESS_ID
}

pub fn path() -> &'static Path {
    lazy_static! {
        static ref PROCESS_PATH: PathBuf =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/"));
    }
    PROCESS_PATH.as_path()
}

pub fn name() -> &'static OsStr {
    lazy_static! {
        static ref PROCESS_NAME: OsString = fs::file_name(path());
    }
    PROCESS_NAME.as_os_str()
}

#[cfg(test)]
mod tests_process {
    use crate::os::process::{id, name, path};
    use std::process::Command;
    use std::{println, string::ToString};

    fn build_commd(s: &str) -> String {
        let mut cmd = "ps -ef |grep ".to_string();
        cmd = cmd + s + "|grep -v grep";
        let output = Command::new("bash").arg("-c").arg(cmd).output().unwrap();
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn test_id() {
        let process_id = id().to_string();
        println!("This process id is {}", process_id);

        let sys_proc = build_commd(&process_id);
        assert!(!sys_proc.is_empty());
    }

    #[test]
    fn test_path() {
        let process_path = path().display().to_string();
        println!("This path is {:#?}", process_path);

        let sys_path = build_commd(&process_path);
        assert!(!sys_path.is_empty());
    }

    #[test]
    fn test_name() {
        let process_name = name().to_string_lossy();
        println!("This name is {:#?}", process_name);

        let sys_name = build_commd(&process_name);
        assert!(!sys_name.is_empty());
    }
}
