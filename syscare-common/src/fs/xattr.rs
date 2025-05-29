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
    ffi::{c_void, CString, OsStr, OsString},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::Path,
    ptr,
};

pub fn getxattr<P, S>(path: P, name: S) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let file_path = CString::new(path.as_ref().as_os_str().as_bytes())?;
    let xattr_name = CString::new(name.as_ref().as_bytes())?;

    /*
     * SAFETY:
     * This libc function is marked 'unsafe' as unchecked buffer may cause overflow.
     * In our implementation, the buffer is checked properly, so that would be safe.
     */
    let buf_size =
        unsafe { nix::libc::getxattr(file_path.as_ptr(), xattr_name.as_ptr(), ptr::null_mut(), 0) };
    if buf_size == -1 {
        return Err(std::io::Error::last_os_error());
    }

    let mut buf = vec![0; buf_size.unsigned_abs()];
    let value_ptr = buf.as_mut_ptr().cast::<c_void>();

    /*
     * SAFETY:
     * This libc function is marked 'unsafe' as unchecked buffer may cause overflow.
     * In our implementation, the buffer is checked properly, so that would be safe.
     */
    let bytes_read = unsafe {
        nix::libc::getxattr(
            file_path.as_ptr(),
            xattr_name.as_ptr(),
            value_ptr,
            buf.len(),
        )
    };
    if bytes_read == -1 {
        return Err(std::io::Error::last_os_error());
    }
    if buf.last() == Some(&0) {
        buf.pop();
    }

    Ok(OsString::from_vec(buf))
}

pub fn setxattr<P, S, T>(path: P, name: S, value: T) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
    T: AsRef<OsStr>,
{
    let file_path = CString::new(path.as_ref().as_os_str().as_bytes())?;
    let xattr_name = CString::new(name.as_ref().as_bytes())?;
    let xattr_value = CString::new(value.as_ref().as_bytes())?;
    let size = xattr_value.to_bytes_with_nul().len();

    /*
     * SAFETY:
     * This libc function is marked 'unsafe' as unchecked buffer may cause overflow.
     * In our implementation, the buffer is checked properly, so that would be safe.
     */
    let ret = unsafe {
        nix::libc::setxattr(
            file_path.as_ptr(),
            xattr_name.as_ptr(),
            xattr_value.as_ptr().cast::<c_void>(),
            size,
            0,
        )
    };
    if ret == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
