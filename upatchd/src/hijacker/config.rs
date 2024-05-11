// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * upatchd is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};

const CC_BINARY: &str = "/usr/bin/cc";
const CXX_BINARY: &str = "/usr/bin/c++";
const GCC_BINARY: &str = "/usr/bin/gcc";
const GXX_BINARY: &str = "/usr/bin/g++";
const AS_BINARY: &str = "/usr/bin/as";

const CC_HIJACKER: &str = "/usr/libexec/syscare/cc-hijacker";
const CXX_HIJACKER: &str = "/usr/libexec/syscare/c++-hijacker";
const GCC_HIJACKER: &str = "/usr/libexec/syscare/gcc-hijacker";
const GXX_HIJACKER: &str = "/usr/libexec/syscare/g++-hijacker";
const AS_HIJACKER: &str = "/usr/libexec/syscare/as-hijacker";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HijackerConfig {
    pub mapping: IndexMap<PathBuf, PathBuf>,
}

impl Default for HijackerConfig {
    fn default() -> Self {
        Self {
            mapping: indexmap! {
                PathBuf::from(CC_BINARY)  => PathBuf::from(CC_HIJACKER),
                PathBuf::from(CXX_BINARY) => PathBuf::from(CXX_HIJACKER),
                PathBuf::from(GCC_BINARY) => PathBuf::from(GCC_HIJACKER),
                PathBuf::from(GXX_BINARY) => PathBuf::from(GXX_HIJACKER),
                PathBuf::from(AS_BINARY)  => PathBuf::from(AS_HIJACKER),
            },
        }
    }
}
