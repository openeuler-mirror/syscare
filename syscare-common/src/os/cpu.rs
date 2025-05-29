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

use std::ffi::OsString;

use num_cpus;

use super::platform;

pub fn arch() -> OsString {
    platform::arch()
}

pub fn num() -> usize {
    num_cpus::get()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_arch() {
        let arch = self::arch();
        println!("cpu arch: {}", arch.to_string_lossy());
        assert!(!arch.is_empty());
    }

    #[test]
    fn test_num() {
        let num = self::num();
        println!("cpu num: {}", num);
        assert_ne!(num, 0);
    }
}
