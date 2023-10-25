use std::{ffi::OsString, os::unix::prelude::OsStringExt};

use anyhow::{anyhow, bail, ensure, Error, Result};

use libc::{c_char, EEXIST, ENOENT, EPERM};
use syscare_abi::PatchStatus;
use syscare_common::util::digest;

use super::{Patch, PatchDriver, UserPatchExt};

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

impl PatchDriver for UserPatchDriver {
    fn check(&self, patch: &Patch) -> Result<()> {
        const ERR_MSG_LEN: usize = 512;

        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let patch_file = &patch_ext.patch_file;

        let real_checksum = digest::file(patch_file).map_err(|e| anyhow!("Upatch: {}", e))?;
        if !patch.checksum.eq(&real_checksum) {
            bail!(
                "Upatch: Patch file \"{}\" checksum failed",
                patch_file.display()
            );
        }

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

    fn status(&self, patch: &Patch) -> Result<PatchStatus> {
        let uuid = patch.uuid.as_str().to_cstring()?;
        let status = unsafe { ffi::upatch_status(uuid.as_ptr()) };

        Ok(status.into())
    }

    fn apply(&self, patch: &Patch) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let patch_file = patch_ext.patch_file.as_path().to_cstring()?;
        let target_elf = patch_ext.target_elf.as_path().to_cstring()?;

        let ret_val = unsafe {
            ffi::upatch_load(
                patch_uuid.as_ptr(),
                target_elf.as_ptr(),
                patch_file.as_ptr(),
            )
        };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Patch status is invalid"),
            ENOENT => bail!("Patch symbol is empty"),
            EEXIST => bail!("Patch is already exist"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn remove(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_remove(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Patch status is invalid"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn active(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_active(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Patch status is invalid"),
            ENOENT => bail!("Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }

    fn deactive(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let ret_val = unsafe { ffi::upatch_deactive(patch_uuid.as_ptr()) };

        match ret_val {
            0 => Ok(()),
            EPERM => bail!("Patch status is invalid"),
            ENOENT => bail!("Cannot find patch entity"),
            _ => Err(Error::from(std::io::Error::from_raw_os_error(ret_val))),
        }
    }
}
