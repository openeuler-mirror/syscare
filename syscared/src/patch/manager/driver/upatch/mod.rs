// SPDX-License-Identifier: Mulan PSL v2
/*
 * Copyright (c) 2024 Huawei Technologies Co., Ltd.
 * syscared is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *         http://license.coscl.org.cn/MulanPSL2
 *
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
 * EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
 * MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, ensure, Result};

use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::{debug, info};
use nix::libc::{c_char, EEXIST, EFAULT, ENOENT, ENOEXEC, EPERM};
use parking_lot::Mutex;
use syscare_abi::PatchStatus;
use syscare_common::util::digest;

use self::{helper::UPatchDriverHelper, monitor::UserPatchMonitor};

use super::{Patch, PatchDriver, PatchOpFlag, UserPatchExt};

mod ffi;
mod helper;
mod monitor;

use ffi::ToCString;

lazy_static! {
    static ref ACTIVE_PATCH_MAP: Mutex<IndexMap<PathBuf, Vec<String>>> =
        Mutex::new(IndexMap::new());
}

pub struct UserPatchDriver {
    monitor: UserPatchMonitor,
}

impl UserPatchDriver {
    fn on_new_process_created(target_elf: &Path) -> Result<()> {
        lazy_static! {
            static ref ELF_PID_MAP: Mutex<IndexMap<PathBuf, Vec<i32>>> =
                Mutex::new(IndexMap::new());
        }

        let active_patch_map = ACTIVE_PATCH_MAP.lock();

        if let Some(patch_list) = active_patch_map.get(target_elf) {
            let mut elf_pid_map = ELF_PID_MAP.lock();

            let new_pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;
            let last_pid_list = elf_pid_map.get(target_elf).cloned().unwrap_or_default();

            let pid_list = new_pid_list
                .iter()
                .filter(|pid| !last_pid_list.contains(pid))
                .cloned()
                .collect::<Vec<_>>();

            if pid_list.is_empty() {
                return Ok(());
            }

            for patch_uuid in patch_list {
                let uuid = patch_uuid.to_cstring()?;
                info!(
                    "Activing patch {{{}}} ({}) for {:?}",
                    patch_uuid,
                    target_elf.display(),
                    pid_list,
                );

                let ret_val =
                    unsafe { ffi::upatch_active(uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };
                match ret_val {
                    0 => continue,
                    EEXIST => continue,
                    EPERM => bail!("Upatch: Operation not permitted"),
                    ENOENT => bail!("Upatch: Cannot find patch entity"),
                    _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(ret_val)),
                }
            }
            elf_pid_map.insert(target_elf.to_owned(), new_pid_list);
        }

        Ok(())
    }

    pub fn new() -> Result<Self> {
        let instance = Self {
            monitor: UserPatchMonitor::new(Self::on_new_process_created)?,
        };
        Ok(instance)
    }
}

impl UserPatchDriver {
    fn check_compatiblity(&self, _patch: &Patch) -> Result<()> {
        Ok(())
    }

    fn check_consistency(&self, patch: &Patch) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let patch_file = &patch_ext.patch_file;

        let real_checksum = digest::file(patch_file).map_err(|e| anyhow!("Upatch: {}", e))?;
        debug!("Target checksum: {}", patch.checksum);
        debug!("Expected checksum: {}", real_checksum);

        ensure!(
            patch.checksum.eq(&real_checksum),
            "Upatch: Patch \"{}\" consistency check failed",
            patch_file.display()
        );

        Ok(())
    }

    fn check_confliction(&self, patch: &Patch) -> Result<()> {
        const ERR_MSG_LEN: usize = 512;

        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let target_elf = patch_ext.target_elf.to_cstring()?;
        let patch_file = patch_ext.patch_file.to_cstring()?;
        let mut msg_buf = vec![0; ERR_MSG_LEN];

        let ret_val = unsafe {
            ffi::upatch_check(
                target_elf.as_ptr(),
                patch_file.as_ptr(),
                msg_buf.as_mut_ptr() as *mut c_char,
                msg_buf.capacity(),
            )
        };
        ensure!(
            ret_val == 0,
            format!(
                "Upatch: {}",
                String::from_utf8_lossy(&msg_buf).trim_end_matches(|c| c == '\0')
            )
        );

        Ok(())
    }
}

impl PatchDriver for UserPatchDriver {
    fn check(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        self.check_compatiblity(patch)?;
        self.check_consistency(patch)?;

        if flag != PatchOpFlag::Force {
            self.check_confliction(patch)?;
        }

        Ok(())
    }

    fn status(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<PatchStatus> {
        let uuid = patch.uuid.as_str().to_cstring()?;
        let status = unsafe { ffi::upatch_status(uuid.as_ptr()) };

        Ok(status.into())
    }

    fn apply(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let patch_file = patch_ext.patch_file.as_path().to_cstring()?;
        let target_elf = patch_ext.target_elf.as_path().to_cstring()?;

        let ret_val = unsafe {
            ffi::upatch_load(
                patch_uuid.as_ptr(),
                target_elf.as_ptr(),
                patch_file.as_ptr(),
                matches!(flag, PatchOpFlag::Force),
            )
        };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Operation not permitted"),
            ENOEXEC => bail!("Upatch: Patch format error"),
            EEXIST => bail!("Upatch: Patch is already exist"),
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(ret_val)),
        }
    }

    fn remove(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_remove(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Operation not permitted"),
            EFAULT => bail!("Upatch: Cannot remove a overrided patch"),
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(ret_val)),
        }
    }

    fn active(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let target_elf = &patch_ext.target_elf;

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;

        let mut active_patch_map = ACTIVE_PATCH_MAP.lock();

        let ret_val =
            unsafe { ffi::upatch_active(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };
        match ret_val {
            0 => {
                let mut need_watch_elf = false;

                match active_patch_map.get_mut(target_elf) {
                    Some(patch_list) => {
                        patch_list.push(patch.uuid.clone());
                    }
                    None => {
                        active_patch_map.insert(target_elf.to_owned(), vec![patch.uuid.clone()]);
                        need_watch_elf = true;
                    }
                };

                if need_watch_elf {
                    drop(active_patch_map);

                    self.monitor.watch_file(target_elf)?;
                }

                Ok(())
            }
            EPERM => bail!("Upatch: Operation not permitted"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(ret_val)),
        }
    }

    fn deactive(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let target_elf = &patch_ext.target_elf;

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;

        let mut active_patch_map = ACTIVE_PATCH_MAP.lock();

        let ret_val =
            unsafe { ffi::upatch_deactive(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };
        match ret_val {
            0 => {
                let mut need_cleanup = false;

                if let Some(patch_list) = active_patch_map.get_mut(target_elf) {
                    patch_list.pop();
                    if patch_list.is_empty() {
                        need_cleanup = true;
                    }
                }

                if need_cleanup {
                    active_patch_map.remove(target_elf);
                    drop(active_patch_map);

                    self.monitor.remove_file(target_elf)?;
                }

                Ok(())
            }
            EPERM => bail!("Upatch: Operation not permitted"),
            EFAULT => bail!("Upatch: Cannot deactive a overrided patch"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => bail!("Upatch: {}", std::io::Error::from_raw_os_error(ret_val)),
        }
    }
}
