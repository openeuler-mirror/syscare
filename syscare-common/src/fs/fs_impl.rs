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
    env,
    ffi::{CStr, OsStr, OsString},
    fs::{File, FileType, Metadata, Permissions, ReadDir},
    io,
    os::{raw::c_void, unix::fs::PermissionsExt},
    path::{Component, Path, PathBuf},
    ptr::null_mut,
};

use crate::ffi::{CStrExt, OsStrExt};

trait RewriteError {
    fn rewrite_err(self, err_msg: String) -> Self;
}

impl<T> RewriteError for io::Result<T> {
    #[inline]
    fn rewrite_err(self, err_msg: String) -> Self {
        self.map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("{}, {}", err_msg, e.to_string().to_lowercase()),
            )
        })
    }
}

/* std::fs functions */
#[inline]
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    std::fs::read(&path).rewrite_err(format!("Cannot read file {}", path.as_ref().display()))
}

#[inline]
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    std::fs::read_to_string(&path)
        .rewrite_err(format!("Cannot read file {}", path.as_ref().display()))
}

#[inline]
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    std::fs::write(&path, contents)
        .rewrite_err(format!("Cannot write file {}", path.as_ref().display()))
}

#[inline]
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_file(&path)
        .rewrite_err(format!("Cannot remove file {}", path.as_ref().display()))
}

#[inline]
pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    std::fs::metadata(&path).rewrite_err(format!("Cannot access {}", path.as_ref().display()))
}

#[inline]
pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    std::fs::symlink_metadata(&path)
        .rewrite_err(format!("Cannot access {}", path.as_ref().display()))
}

#[inline]
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    std::fs::rename(&from, &to).rewrite_err(format!(
        "Cannot rename {} to {}",
        from.as_ref().display(),
        to.as_ref().display()
    ))
}

#[inline]
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    std::fs::copy(&from, &to).rewrite_err(format!(
        "Cannot copy {} to {}",
        from.as_ref().display(),
        to.as_ref().display()
    ))
}

#[inline]
pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    std::fs::hard_link(original.as_ref(), link.as_ref()).rewrite_err(format!(
        "Cannot link {} to {}",
        original.as_ref().display(),
        link.as_ref().display()
    ))
}

#[inline]
pub fn soft_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    // std::fs::soft_link() is deprecated, use std::os::unix::fs::symlink instead
    std::os::unix::fs::symlink(original.as_ref(), link.as_ref()).rewrite_err(format!(
        "Cannot link {} to {}",
        original.as_ref().display(),
        link.as_ref().display()
    ))
}

#[inline]
pub fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    std::fs::read_link(&path).rewrite_err(format!(
        "Cannot read symbol link {}",
        path.as_ref().display(),
    ))
}

#[inline]
pub fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    std::fs::canonicalize(&path)
        .rewrite_err(format!("Cannot canonicalize {}", path.as_ref().display()))
}

#[inline]
pub fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::create_dir(&path).rewrite_err(format!(
        "Cannot create directory {}",
        path.as_ref().display(),
    ))
}

#[inline]
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::create_dir_all(&path).rewrite_err(format!(
        "Cannot create directory {}",
        path.as_ref().display(),
    ))
}

#[inline]
pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_dir(&path).rewrite_err(format!(
        "Cannot remove directory {}",
        path.as_ref().display(),
    ))
}

#[inline]
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::remove_dir_all(&path).rewrite_err(format!(
        "Cannot remove directory {}",
        path.as_ref().display(),
    ))
}

#[inline]
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    std::fs::read_dir(&path)
        .rewrite_err(format!("Cannot read directory {}", path.as_ref().display()))
}

#[inline]
pub fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    std::fs::set_permissions(&path, perm.clone()).rewrite_err(format!(
        "Cannot set path {} to permission {:05o}",
        path.as_ref().display(),
        perm.mode()
    ))
}

/* Extended functions */
pub fn create_file<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::create(&path).rewrite_err(format!("Cannot create file {}", path.as_ref().display()))
}

pub fn open_file<P: AsRef<Path>>(path: P) -> io::Result<File> {
    File::open(&path).rewrite_err(format!("Cannot open file {}", path.as_ref().display()))
}

pub fn file_name<P: AsRef<Path>>(path: P) -> OsString {
    path.as_ref()
        .file_name()
        .map(OsStr::to_os_string)
        .unwrap_or_default()
}

pub fn file_ext<P: AsRef<Path>>(path: P) -> OsString {
    path.as_ref()
        .extension()
        .map(OsStr::to_os_string)
        .unwrap_or_default()
}

pub fn getxattr<P, S>(path: P, name: S) -> std::io::Result<OsString>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let file_path = path.as_ref().to_cstring()?;
    let xattr_name = name.as_ref().to_cstring()?;

    /*
     * SAFETY:
     * This libc function is marked 'unsafe' as unchecked buffer may cause overflow.
     * In our implementation, the buffer is checked properly, so that would be safe.
     */
    let mut ret =
        unsafe { nix::libc::getxattr(file_path.as_ptr(), xattr_name.as_ptr(), null_mut(), 0) };
    if ret == -1 {
        return Err(std::io::Error::last_os_error()).rewrite_err(format!(
            "Cannot get path {} xattr {}",
            file_path.to_string_lossy(),
            xattr_name.to_string_lossy()
        ));
    }

    let mut buf = vec![0; ret.unsigned_abs()];
    let value_ptr = buf.as_mut_ptr().cast::<c_void>();

    /*
     * SAFETY:
     * This libc function is marked 'unsafe' as unchecked buffer may cause overflow.
     * In our implementation, the buffer is checked properly, so that would be safe.
     */
    ret = unsafe {
        nix::libc::getxattr(
            file_path.as_ptr(),
            xattr_name.as_ptr(),
            value_ptr,
            buf.len(),
        )
    };
    if ret == -1 {
        return Err(std::io::Error::last_os_error()).rewrite_err(format!(
            "Cannot get path {} xattr {}",
            file_path.to_string_lossy(),
            xattr_name.to_string_lossy(),
        ));
    }

    let value = CStr::from_bytes_with_nul(&buf[0..ret.unsigned_abs()])
        .unwrap_or_default()
        .to_os_string();

    Ok(value)
}

pub fn setxattr<P, S, T>(path: P, name: S, value: T) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
    T: AsRef<OsStr>,
{
    let file_path = path.as_ref().to_cstring()?;
    let xattr_name = name.as_ref().to_cstring()?;
    let xattr_value = value.as_ref().to_cstring()?;
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
        return Err(std::io::Error::last_os_error()).rewrite_err(format!(
            "Cannot set {} xattr {}",
            file_path.to_string_lossy(),
            xattr_name.to_string_lossy()
        ));
    }

    Ok(())
}

pub fn normalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let mut new_path = PathBuf::new();

    let orig_path = path.as_ref();
    if orig_path.as_os_str().is_empty() {
        return Ok(new_path);
    }

    if orig_path.is_relative() {
        new_path.push(env::current_dir()?);
    }

    for component in orig_path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                new_path.pop();
                if !new_path.has_root() {
                    new_path.push(Component::RootDir);
                }
            }
            _ => {
                new_path.push(component);
            }
        }
    }

    Ok(new_path)
}

#[derive(Clone, Copy)]
pub struct TraverseOptions {
    pub recursive: bool,
}

pub fn traverse<P, F>(
    directory: P,
    options: TraverseOptions,
    predicate: F,
) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
    F: Fn(&FileType, &Path) -> bool + Copy,
{
    let mut results = Vec::new();
    let mut subdirs = Vec::new();

    for dir_entry in read_dir(directory)?.flatten() {
        let file_type = dir_entry.file_type()?;
        let file_path = dir_entry.path();

        if predicate(&file_type, &file_path) {
            results.push(file_path.clone());
        }
        if options.recursive && file_type.is_dir() {
            subdirs.push(file_path);
        }
    }

    for subdir in subdirs {
        results.extend(traverse(subdir, options, predicate)?);
    }

    Ok(results)
}

pub fn list_dirs<P>(directory: P, options: TraverseOptions) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    traverse(directory, options, |file_type, _| file_type.is_dir())
}

pub fn list_files<P>(directory: P, options: TraverseOptions) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    traverse(directory, options, |file_type, _| file_type.is_file())
}

pub fn list_files_by_ext<P, S>(
    directory: P,
    ext: S,
    options: TraverseOptions,
) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    traverse(directory, options, |file_type, file_path| {
        if !file_type.is_file() {
            return false;
        }
        return file_path
            .extension()
            .map(|s| s == ext.as_ref())
            .unwrap_or(false);
    })
}

pub fn list_symlinks<P>(directory: P, options: TraverseOptions) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    traverse(directory, options, |file_type, _| file_type.is_symlink())
}

#[derive(Clone, Copy)]
pub struct FindOptions {
    pub fuzz: bool,
    pub recursive: bool,
}

pub fn find<P, F>(directory: P, options: FindOptions, predicate: F) -> io::Result<Option<PathBuf>>
where
    P: AsRef<Path>,
    F: Fn(&FileType, &Path) -> bool + Copy,
{
    let mut subdirs = Vec::new();

    for dir_entry in read_dir(directory)?.flatten() {
        let file_type = dir_entry.file_type()?;
        let file_path = dir_entry.path();

        if predicate(&file_type, &file_path) {
            return Ok(Some(file_path));
        }
        if options.recursive && file_type.is_dir() {
            subdirs.push(file_path);
        }
    }

    for subdir in subdirs {
        if let Some(path) = find(subdir, options, predicate)? {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

pub fn find_dir<P, S>(directory: P, name: S, options: FindOptions) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let result = find(&directory, options, |file_type, file_path| -> bool {
        if !file_type.is_dir() {
            return false;
        }
        if let Some(file_name) = file_path.file_name() {
            if options.fuzz {
                return file_name.contains(name.as_ref());
            } else {
                return file_name == name.as_ref();
            }
        }
        false
    })?;

    result.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Cannot find directory {} from {}",
                name.as_ref().to_string_lossy(),
                directory.as_ref().display()
            ),
        )
    })
}

pub fn find_file<P, S>(directory: P, name: S, options: FindOptions) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let result = find(&directory, options, |file_type, file_path| -> bool {
        if !file_type.is_file() {
            return false;
        }
        if let Some(file_name) = file_path.file_name() {
            if options.fuzz {
                return file_name.contains(name.as_ref());
            } else {
                return file_name == name.as_ref();
            }
        }
        false
    })?;

    result.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Cannot find file {} from {}",
                name.as_ref().to_string_lossy(),
                directory.as_ref().display()
            ),
        )
    })
}

pub fn find_file_by_ext<P, S>(directory: P, ext: S, options: FindOptions) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let result = find(
        &directory,
        options,
        |file_type: &FileType, file_path: &Path| -> bool {
            if !file_type.is_file() {
                return false;
            }
            if let Some(file_name) = file_path.extension() {
                if file_name == ext.as_ref() {
                    return true;
                }
            }
            false
        },
    )?;

    result.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Cannot find file \"*.{}\" from {}",
                ext.as_ref().to_string_lossy(),
                directory.as_ref().display()
            ),
        )
    })
}

pub fn find_symlink<P, S>(directory: P, name: S, options: FindOptions) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let result = find(&directory, options, |file_type, file_path| -> bool {
        if !file_type.is_symlink() {
            return false;
        }
        if let Some(file_name) = file_path.file_name() {
            if options.fuzz {
                return file_name.contains(name.as_ref());
            } else {
                return file_name == name.as_ref();
            }
        }
        false
    })?;

    result.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Cannot find symlink {} from {}",
                name.as_ref().to_string_lossy(),
                directory.as_ref().display()
            ),
        )
    })
}

pub fn copy_dir_contents<P: AsRef<Path>, Q: AsRef<Path>>(src_dir: P, dst_dir: Q) -> io::Result<()> {
    for src_file in list_files(src_dir, TraverseOptions { recursive: true })? {
        let dst_file = dst_dir
            .as_ref()
            .join(src_file.file_name().unwrap_or_default());

        copy(src_file, dst_file)?;
    }

    Ok(())
}

pub fn glob<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    const CURRENT_DIR: Component = Component::CurDir;
    const WILDCARD_ONE: char = '?';
    const WILDCARD_ALL: char = '*';
    const WILDCARD_RECURSIVE: &str = "**";

    fn match_name(name: &OsStr, pattern: &OsStr) -> bool {
        let name: Vec<_> = name.chars().collect();
        let pattern: Vec<_> = pattern.chars().collect();

        let (mut name_idx, mut patt_idx, mut star_idx, mut match_idx) = (0, 0, None, 0);

        while name_idx < name.len() {
            if patt_idx < pattern.len()
                && (pattern[patt_idx] == WILDCARD_ONE || pattern[patt_idx] == name[name_idx])
            {
                name_idx += 1;
                patt_idx += 1;
            } else if patt_idx < pattern.len() && pattern[patt_idx] == WILDCARD_ALL {
                star_idx = Some(patt_idx);
                match_idx = name_idx;
                patt_idx += 1;
            } else if let Some(star) = star_idx {
                patt_idx = star + 1;
                match_idx += 1;
                name_idx = match_idx;
            } else {
                return false;
            }
        }

        while patt_idx < pattern.len() && pattern[patt_idx] == WILDCARD_ALL {
            patt_idx += 1;
        }

        patt_idx == pattern.len()
    }

    let pattern = path.as_ref();
    let base_dir = if pattern.is_relative() && !pattern.starts_with(CURRENT_DIR.as_os_str()) {
        PathBuf::from(CURRENT_DIR.as_os_str())
    } else {
        PathBuf::new()
    };
    let components = pattern.components().collect::<Vec<_>>();

    let mut result = Vec::new();
    let mut stack = vec![(base_dir, 0)];

    while let Some((mut curr_dir, mut comp_idx)) = stack.pop() {
        let comp_len = components.len();
        while comp_idx < comp_len {
            if let Component::Normal(pattern) = components[comp_idx] {
                if pattern == WILDCARD_RECURSIVE {
                    if let Ok(read_dir) = self::read_dir(&curr_dir) {
                        for dir_entry in read_dir.flatten() {
                            let next_path = dir_entry.path();
                            if next_path.is_dir() {
                                stack.push((next_path.clone(), comp_idx));
                            }
                            if comp_idx + 1 == comp_len {
                                result.push(next_path);
                                continue;
                            }
                            stack.push((next_path, comp_idx + 1));
                        }
                    }
                    break;
                } else if pattern.contains(WILDCARD_ONE) || pattern.contains(WILDCARD_ALL) {
                    if let Ok(read_dir) = self::read_dir(&curr_dir) {
                        for dir_entry in read_dir.flatten() {
                            let next_path = dir_entry.path();
                            let file_name = next_path.file_name().unwrap_or_default();
                            if !match_name(file_name, pattern) {
                                continue;
                            }
                            if comp_idx + 1 == comp_len {
                                result.push(next_path);
                                continue;
                            }
                            if next_path.is_dir() {
                                stack.push((next_path, comp_idx + 1));
                            }
                        }
                    }
                    break;
                }
            }

            curr_dir.push(components[comp_idx]);
            if curr_dir.exists() && (comp_idx + 1 == comp_len) {
                result.push(curr_dir.clone());
                break;
            }
            comp_idx += 1;
        }
    }

    Ok(result)
}

pub fn sync() {
    nix::unistd::sync()
}
