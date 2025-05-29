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
    collections::HashSet,
    ffi::{CStr, CString, OsStr, OsString},
    fs::File,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use log::{error, trace};
use nix::{errno::Errno, kmod};
use object::{Object, ObjectSection};

use super::{platform, selinux};
use crate::fs;

const SYS_MODULE_DIR: &str = "/sys/module";

pub type KernelModuleInfo = ModuleInfo;
pub type KernelModuleGuard = ModuleGuard;

#[derive(Debug)]
pub struct ModuleInfo {
    pub name: OsString,
    pub depends: HashSet<OsString>,
    pub module_path: PathBuf,
}

#[derive(Debug)]
pub struct ModuleGuard(ModuleInfo);

impl Drop for ModuleGuard {
    fn drop(&mut self) {
        if !self.0.module_path.exists() {
            return;
        }
        if let Err(e) = self::remove_module(&self.0.name) {
            error!(
                "Failed to remove kernel module '{}', {}",
                self.0.name.to_string_lossy(),
                e.to_string().to_lowercase()
            );
        }
    }
}

pub fn version() -> OsString {
    platform::release()
}

pub fn list_modules() -> std::io::Result<HashSet<OsString>> {
    const LIST_OPTIONS: fs::TraverseOptions = fs::TraverseOptions { recursive: false };

    let modules = fs::list_dirs(SYS_MODULE_DIR, LIST_OPTIONS)?
        .into_iter()
        .filter_map(|path| path.file_name().map(OsStr::to_os_string))
        .collect();
    Ok(modules)
}

pub fn relable_module_file<P: AsRef<Path>>(file_path: P) -> std::io::Result<()> {
    const KMOD_FILE_TYPE: &str = "modules_object_t";

    let file_path = file_path.as_ref();
    let mut context = selinux::get_security_context(file_path)?;
    if context.get_type() == KMOD_FILE_TYPE {
        return Ok(());
    }

    context.set_type(KMOD_FILE_TYPE)?;
    selinux::set_security_context(file_path, &context)?;

    Ok(())
}

pub fn module_info<P: AsRef<Path>>(file_path: P) -> std::io::Result<ModuleInfo> {
    const MODINFO_SECTION_NAME: &str = ".modinfo";
    const MODINFO_NAME_PREFIX: &[u8] = b"name=";
    const MODINFO_DEPENDS_PREFIX: &[u8] = b"depends=";

    let file_path = file_path.as_ref().to_path_buf();
    let mmap = fs::mmap(&file_path)?;
    let file = object::File::parse(&*mmap).map_err(|_| Errno::ENOEXEC)?;
    if file.format() != object::BinaryFormat::Elf {
        return Err(std::io::Error::from(Errno::ENOEXEC));
    }

    let data = file
        .section_by_name(MODINFO_SECTION_NAME)
        .and_then(|section| section.data().ok())
        .ok_or(Errno::ENOEXEC)?;
    let name = data
        .split(|b| *b == b'\0')
        .find_map(|entry| entry.strip_prefix(MODINFO_NAME_PREFIX))
        .map(|slice| OsStr::from_bytes(slice).to_os_string())
        .ok_or(Errno::ENOEXEC)?;
    let depends = data
        .split(|b| *b == b'\0')
        .find_map(|entry| entry.strip_prefix(MODINFO_DEPENDS_PREFIX))
        .unwrap_or_default()
        .split(|b| *b == b',')
        .filter(|b| !b.is_empty())
        .map(|b| OsStr::from_bytes(b).to_os_string())
        .collect();
    let module_path = Path::new(SYS_MODULE_DIR).join(&name);

    let kmod = ModuleInfo {
        name,
        depends,
        module_path,
    };
    Ok(kmod)
}

pub fn insert_module<P: AsRef<Path>>(file_path: P) -> std::io::Result<()> {
    const PARAM_VALUES: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"\0") };

    let file_path = file_path.as_ref();
    trace!("Inserting kernel module '{}'...", file_path.display());

    let file = File::open(file_path)?;
    kmod::finit_module(&file, PARAM_VALUES, kmod::ModuleInitFlags::empty())?;

    Ok(())
}

pub fn insert_module_guarded<P: AsRef<Path>>(file_path: P) -> std::io::Result<ModuleGuard> {
    let file_path = file_path.as_ref();

    let modinfo = self::module_info(file_path)?;
    if !modinfo.module_path.exists() {
        self::insert_module(file_path)?;
    }

    Ok(ModuleGuard(modinfo))
}

pub fn remove_module<S: AsRef<OsStr>>(module_name: S) -> std::io::Result<()> {
    let module_name = module_name.as_ref();
    trace!(
        "Removing kernel module '{}'...",
        module_name.to_string_lossy()
    );

    let name = CString::new(module_name.as_bytes())?;
    kmod::delete_module(&name, kmod::DeleteModuleFlags::O_NONBLOCK)?;

    Ok(())
}
