// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscare-abi is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackageType {
    SourcePackage,
    BinaryPackage,
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub kind: PackageType,
    pub arch: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub license: String,
    pub source_pkg: String,
}

impl PackageInfo {
    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.name, self.version, self.release, self.arch
        )
    }
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "name:    {}", self.name)?;
        writeln!(f, "type:    {}", self.kind)?;
        writeln!(f, "arch:    {}", self.arch)?;
        writeln!(f, "epoch:   {}", self.epoch)?;
        writeln!(f, "version: {}", self.version)?;
        writeln!(f, "release: {}", self.release)?;
        write!(f, "license: {}", self.license)?;

        Ok(())
    }
}

#[test]
fn test_packageinfo () {
    let packinfo = PackageInfo {
        name: "testpackage".to_string(),
        kind: PackageType::BinaryPackage,
        arch: "x86_64".to_string(),
        epoch: "None".to_string(),
        version: "1".to_string(),
        release: "1".to_string(),
        license: "GPL".to_string(),
        source_pkg: "source".to_string(),
    };
    assert_eq!(packinfo.short_name(), "testpackage-1-1".to_string());
    assert_eq!(packinfo.full_name(), "testpackage-1-1.x86_64".to_string());
}
