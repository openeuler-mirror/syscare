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

use std::{
    ffi::{CStr, CString, FromBytesWithNulError, OsStr, OsString},
    os::unix::{ffi::OsStringExt, prelude::OsStrExt},
    path::{Path, PathBuf},
};

pub trait CStrExt: AsRef<CStr> {
    fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes(self.as_ref().to_bytes())
    }

    fn as_path(&self) -> &Path {
        Path::new(self.as_os_str())
    }

    fn to_os_string(&self) -> OsString {
        OsString::from_vec(self.as_ref().to_bytes().to_vec())
    }

    fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_os_string())
    }

    fn from_bytes_with_next_nul(bytes: &[u8]) -> Result<&CStr, FromBytesWithNulError> {
        let nul_pos = bytes.iter().position(|b| b == &b'\0').unwrap_or(0);
        let cstr_bytes = &bytes[..=nul_pos];

        CStr::from_bytes_with_nul(cstr_bytes)
    }
}

impl CStrExt for CStr {}
impl CStrExt for &CStr {}
impl CStrExt for CString {}
impl CStrExt for &CString {}

#[test]
fn test_cstr() {
    use std::ffi::CString;

    let path = Path::new("/tmp/test");
    let cstring = CString::new("/tmp/test").unwrap();

    assert_eq!(path.as_os_str().as_bytes(), cstring.to_bytes());
    assert_ne!(path.as_os_str().as_bytes(), cstring.to_bytes_with_nul());

    println!("Testing trait CStrExt::as_os_str...");
    assert_eq!(path.as_os_str(), cstring.as_os_str());

    println!("Testing trait CStrExt::as_path...");
    assert_eq!(path, cstring.as_path());

    println!("Testing trait CStrExt::to_os_string...");
    assert_eq!(path.as_os_str().to_os_string(), cstring.to_os_string());

    println!("Testing trait CStrExt::to_path_buf...");
    assert_eq!(path.to_path_buf(), cstring.to_path_buf());
}
