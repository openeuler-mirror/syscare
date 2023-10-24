use anyhow::{anyhow, bail, ensure, Result};

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
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();
        let patch_file = patch_ext.patch_file.as_path();

        let real_checksum = digest::file(patch_file).map_err(|e| anyhow!("Upatch: {}", e))?;
        if !patch.checksum.eq(&real_checksum) {
            bail!(
                "Upatch: Patch file \"{}\" checksum failed",
                patch_file.display()
            );
        }

        Ok(())
    }

    fn status(&self, patch: &Patch) -> Result<PatchStatus> {
        let uuid = patch.uuid.as_str().to_cstring()?;
        unsafe {
            let status = ffi::upatch_status(uuid.as_ptr());
            Ok(status.into())
        }
    }

    fn apply(&self, patch: &Patch) -> Result<()> {
        let patch_ext: &UserPatchExt = (&patch.info_ext).into();

        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        let patch_file = patch_ext.patch_file.as_path().to_cstring()?;
        let target_elf = patch_ext.target_elf.as_path().to_cstring()?;

        unsafe {
            let ret_val = ffi::upatch_load(
                patch_uuid.as_ptr(),
                target_elf.as_ptr(),
                patch_file.as_ptr(),
            );
            ensure!(ret_val == 0, std::io::Error::from_raw_os_error(ret_val));
        }
        Ok(())
    }

    fn remove(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        unsafe {
            let ret_val = ffi::upatch_remove(patch_uuid.as_ptr());
            ensure!(ret_val == 0, std::io::Error::from_raw_os_error(ret_val));
        }
        Ok(())
    }

    fn active(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        unsafe {
            let ret_val = ffi::upatch_active(patch_uuid.as_ptr());
            ensure!(ret_val == 0, std::io::Error::from_raw_os_error(ret_val));
        }
        Ok(())
    }

    fn deactive(&self, patch: &Patch) -> Result<()> {
        let patch_uuid = patch.uuid.as_str().to_cstring()?;
        unsafe {
            let ret_val = ffi::upatch_deactive(patch_uuid.as_ptr());
            ensure!(ret_val == 0, std::io::Error::from_raw_os_error(ret_val));
        }
        Ok(())
    }
}
