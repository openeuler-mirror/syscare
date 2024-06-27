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

use std::ffi::OsStr;

use lazy_static::lazy_static;

use nix::{
    sched::{sched_getaffinity, CpuSet},
    unistd::getpid,
};

use super::platform;

pub fn arch() -> &'static OsStr {
    platform::arch()
}

pub fn num() -> usize {
    lazy_static! {
        static ref CPU_NUM: usize = {
            let cpu_set = sched_getaffinity(getpid()).unwrap_or_default();
            let mut cpu_count = 0;
            for i in 0..CpuSet::count() {
                if cpu_set.is_set(i).unwrap_or_default() {
                    cpu_count += 1;
                }
            }
            cpu_count
        };
    }
    *CPU_NUM
}

#[test]
fn test() {
    println!("arch: {}", arch().to_string_lossy());
    println!("num: {}", num())
}
