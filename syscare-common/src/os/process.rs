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
            std::env::current_exe().expect("Read process path failed");
    }
    PROCESS_PATH.as_path()
}

pub fn name() -> &'static OsStr {
    lazy_static! {
        static ref PROCESS_NAME: OsString = fs::file_name(path());
    }
    PROCESS_NAME.as_os_str()
}
