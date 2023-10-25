use std::{
    ffi::OsString,
    os::unix::prelude::OsStringExt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Error, Result};

use libc::{c_char, EEXIST, EFAULT, ENOENT, EPERM};
use log::warn;
use syscare_abi::PatchStatus;
use syscare_common::util::{
    digest,
    fs::{self, TraverseOptions},
};

use super::{Patch, PatchDriver, PatchOpFlag, UserPatchExt};

mod ffi;
mod guard;

use ffi::ToCString;
pub use guard::*;

pub struct UserPatchDriver {
    _guard: UPatchDriverKmodGuard,
}

impl UserPatchDriver {
    pub fn new() -> Result<Self> {
        Ok(Self {
            _guard: UPatchDriverKmodGuard::new()?,
        })
    }
}

impl UserPatchDriver {
    fn find_pid_by_elf_file(&self, target_elf: &Path) -> Result<Vec<i32>> {
        let pid_list = fs::list_dirs("/proc", TraverseOptions { recursive: false })?
            .into_iter()
            .filter_map(|dir_path| {
                fs::file_name(dir_path)
                    .to_string_lossy()
                    .as_ref()
                    .parse::<i32>()
                    .ok()
            })
            .filter(|pid| {
                fs::read_link(PathBuf::from(format!("/proc/{}/exe", pid)))
                    .map(|exe_path| exe_path == target_elf)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        Ok(pid_list)
    }
}

impl PatchDriver for UserPatchDriver {
    fn check(&self, patch: &Patch, flag: PatchOpFlag) -> Result<()> {
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

    fn remove(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_remove(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Patch status is invalid"),
            EFAULT => bail!("Upatch: Cannot remove a overrided patch"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn active(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = self.find_pid_by_elf_file(&patch_ext.target_elf)?;
        let ret_val =
            unsafe { ffi::upatch_active(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Patch status is invalid"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn deactive(&self, patch: &Patch, _flag: PatchOpFlag) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let pid_list = self.find_pid_by_elf_file(&patch_ext.target_elf)?;
        let ret_val =
            unsafe { ffi::upatch_deactive(patch_uuid.as_ptr(), pid_list.as_ptr(), pid_list.len()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Upatch: Patch status is invalid"),
            EFAULT => bail!("Upatch: Cannot deactive a overrided patch"),
            ENOENT => bail!("Upatch: Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }
}
