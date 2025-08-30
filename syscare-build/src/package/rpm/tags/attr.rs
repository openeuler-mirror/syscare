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
pub struct RpmDefAttr {
    pub file_mode: u32,
    pub user: String,
    pub group: String,
    pub dir_mode: u32,
}

impl std::fmt::Display for RpmDefAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "%defattr({:o},{},{},{:o})",
            self.file_mode, self.user, self.group, self.dir_mode
        ))
    }
}

#[test]
fn test() {
    let def_attr = RpmDefAttr {
        file_mode: 0o755,
        user: String::from("root"),
        group: String::from("nobody"),
        dir_mode: 0o755,
    };
    println!("RpmDefAttr::new()\n{}\n", def_attr);
    assert_eq!(def_attr.to_string(), "%defattr(755,root,nobody,755)");
}
