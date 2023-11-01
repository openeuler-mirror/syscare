use std::{
    ffi::OsString,
    os::unix::prelude::OsStringExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, bail, ensure, Error, Result};

use indexmap::IndexMap;
use lazy_static::lazy_static;
use libc::{c_char, EEXIST, EFAULT, ENOENT, EPERM};
use log::{info, warn};
use parking_lot::Mutex;
use syscare_abi::PatchStatus;
use syscare_common::util::digest;

use self::{helper::UPatchDriverHelper, monitor::UserPatchMonitor};

use super::{Patch, PatchDriver, PatchOpFlag, UserPatchExt};

mod ffi;
mod guard;
mod helper;
mod monitor;

use ffi::ToCString;
pub use guard::*;

lazy_static! {
    static ref ACTIVE_PATCH_MAP: Mutex<IndexMap<PathBuf, Vec<Arc<Patch>>>> =
        Mutex::new(IndexMap::new());
}

pub struct UserPatchDriver {
    _guard: UPatchDriverKmodGuard,
    monitor: UserPatchMonitor,
}

impl UserPatchDriver {
    fn on_new_process_created(target_elf: &Path) -> Result<()> {
        info!("New process started \"{}\"", target_elf.display());
        if let Some(stack) = ACTIVE_PATCH_MAP.lock().get(target_elf) {
            for patch in stack {
                let patch_uuid = patch.uuid.as_str().to_cstring()?;
                let pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;

                info!(
                    "Activing patch {{{}}} for \"{}\"",
                    patch.uuid,
                    target_elf.display()
                );
                let ret_val = unsafe {
                    ffi::upatch_active(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len())
                };
                match ret_val {
                    0 => continue,
                    EEXIST => continue,
                    EPERM => bail!("Upatch: Patch status is invalid"),
                    ENOENT => bail!("Upatch: Cannot find patch entity"),
                    _ => return Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
                }
            }
        }

        Ok(())
    }

    pub fn new() -> Result<Self> {
        let instance = Self {
            _guard: UPatchDriverKmodGuard::new()?,
            monitor: UserPatchMonitor::new(Self::on_new_process_created)?,
        };
        Ok(instance)
    }
}

impl PatchDriver for UserPatchDriver {
    fn check(&self, patch: Arc<Patch>, flag: PatchOpFlag) -> Result<()> {
        const ERR_MSG_LEN: usize = 512;

        if flag == PatchOpFlag::SkipCheck {
            warn!("Skipped patch \"{}\" check", patch);
            return Ok(());
        }

        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let patch_file = &patch_ext.patch_file;

        let real_checksum = digest::file(patch_file).map_err(|e| anyhow!("Upatch: {}", e))?;
        ensure!(
            patch.checksum.eq(&real_checksum),
            "Upatch: Patch file \"{}\" checksum failed",
            patch_file.display()
        );

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
            OsString::from_vec(msg_buf).to_string_lossy().to_string()
        );

        Ok(())
    }

    fn status(&self, patch: Arc<Patch>, _flag: PatchOpFlag) -> Result<PatchStatus> {
        let uuid = patch.uuid.as_str().to_cstring()?;
        let status = unsafe { ffi::upatch_status(uuid.as_ptr()) };

        Ok(status.into())
    }

    fn apply(&self, patch: Arc<Patch>, flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let patch_file = patch_ext.patch_file.as_path().to_cstring()?;
        let target_elf = patch_ext.target_elf.as_path().to_cstring()?;

        let ret_val = unsafe {
            ffi::upatch_load(
                patch_uuid.as_ptr(),
                target_elf.as_ptr(),
                patch_file.as_ptr(),
                matches!(flag, PatchOpFlag::SkipCheck),
            )
        };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Patch status is invalid"),
            ENOENT => bail!("Upatch: Patch symbol is empty"),
            EEXIST => bail!("Upatch: Patch is already exist"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn remove(&self, patch: Arc<Patch>, _flag: PatchOpFlag) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_remove(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Patch status is invalid"),
            EFAULT => bail!("Upatch: Cannot remove a overrided patch"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn active(&self, patch: Arc<Patch>, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let target_elf = &patch_ext.target_elf;

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;
        let ret_val =
            unsafe { ffi::upatch_active(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };

        match ret_val {
            0 => {
                let mut active_patch_map = ACTIVE_PATCH_MAP.lock();
                match active_patch_map.get_mut(target_elf) {
                    Some(patch_list) => {
                        patch_list.push(patch.clone());
                    }
                    None => {
                        active_patch_map.insert(target_elf.to_owned(), vec![patch.clone()]);
                    }
                };

                self.monitor.watch_file(target_elf)?;
                Ok(())
            }
            EPERM => bail!("Upatch: Patch status is invalid"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn deactive(&self, patch: Arc<Patch>, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let target_elf = &patch_ext.target_elf;

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = UPatchDriverHelper::find_target_elf_pid(target_elf)?;
        let ret_val =
            unsafe { ffi::upatch_deactive(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };

        match ret_val {
            0 => {
                let mut actived_patch = ACTIVE_PATCH_MAP.lock();
                if let Some(patch_list) = actived_patch.get_mut(target_elf) {
                    patch_list.pop();
                }

                self.monitor.remove_file(target_elf)?;
                Ok(())
            }
            EPERM => bail!("Upatch: Patch status is invalid"),
            EFAULT => bail!("Upatch: Cannot deactive a overrided patch"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }
}
