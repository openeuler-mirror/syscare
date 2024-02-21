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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RpmDefine {
    pub name: String,
    pub value: String,
}

impl std::fmt::Display for RpmDefine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("%define {} {}", self.name, self.value))
    }
}

#[test]
fn test() {
    let define = RpmDefine {
        name: String::from("macro_test"),
        value: String::from("1"),
    };
    println!("RpmMacro::Define\n{}\n", define);
    assert_eq!(define.to_string(), "%define macro_test 1");
}
