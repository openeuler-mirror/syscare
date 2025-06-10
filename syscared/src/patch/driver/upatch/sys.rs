use std::{
    ffi::CString,
    os::unix::{
        ffi::OsStrExt,
        io::{AsRawFd, RawFd},
    },
    path::Path,
};

use log::debug;
use nix::errno::Errno;

use syscare_abi::PatchStatus;

mod ffi {
    use nix::ioctl_write_ptr;
    use std::ffi::c_char;

    const UPATCH_MAGIC: u8 = 0xE5;

    const UPATCH_LOAD: u8 = 0x01;
    const UPATCH_ACTIVE: u8 = 0x02;
    const UPATCH_DEACTIVE: u8 = 0x03;
    const UPATCH_REMOVE: u8 = 0x04;
    const UPATCH_STATUS: u8 = 0x05;

    pub const UPATCH_STATUS_NOT_APPLIED: i32 = 1;
    pub const UPATCH_STATUS_DEACTIVED: i32 = 2;
    pub const UPATCH_STATUS_ACTIVED: i32 = 3;

    #[repr(C)]
    pub struct PatchLoadRequest {
        pub patch_file: *const c_char,
        pub target_elf: *const c_char,
    }

    ioctl_write_ptr!(
        ioctl_load_patch,
        UPATCH_MAGIC,
        UPATCH_LOAD,
        PatchLoadRequest
    );
    ioctl_write_ptr!(ioctl_active_patch, UPATCH_MAGIC, UPATCH_ACTIVE, c_char);
    ioctl_write_ptr!(ioctl_deactive_patch, UPATCH_MAGIC, UPATCH_DEACTIVE, c_char);
    ioctl_write_ptr!(ioctl_remove_patch, UPATCH_MAGIC, UPATCH_REMOVE, c_char);
    ioctl_write_ptr!(ioctl_get_patch_status, UPATCH_MAGIC, UPATCH_STATUS, c_char);
}

pub fn get_patch_status<P>(fd: RawFd, patch_file: P) -> std::io::Result<PatchStatus>
where
    P: AsRef<Path>,
{
    let patch_file = patch_file.as_ref();
    debug!(
        "Upatch: Ioctl {{ fd: {}, cmd: {}, data: {} }}",
        fd,
        stringify!(UPATCH_STATUS),
        patch_file.display(),
    );

    let patch_cstr = CString::new(patch_file.as_os_str().as_bytes())?;
    let status_code = unsafe { ffi::ioctl_get_patch_status(fd, patch_cstr.as_ptr())? };
    let status = match status_code {
        ffi::UPATCH_STATUS_NOT_APPLIED => PatchStatus::NotApplied,
        ffi::UPATCH_STATUS_DEACTIVED => PatchStatus::Deactived,
        ffi::UPATCH_STATUS_ACTIVED => PatchStatus::Actived,
        _ => return Err(std::io::Error::from(Errno::EINVAL)),
    };

    Ok(status)
}

pub fn load_patch<P, Q>(ioctl_dev: RawFd, patch_file: Q, target_elf: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let ioctl_fd = ioctl_dev.as_raw_fd();
    let patch_file = patch_file.as_ref();
    let target_elf = target_elf.as_ref();
    debug!(
        "Upatch: Ioctl {{ fd: {}, cmd: {}, data: {{ {}, {} }} }}",
        ioctl_fd,
        stringify!(UPATCH_LOAD),
        patch_file.display(),
        target_elf.display(),
    );

    let patch_cstr = CString::new(patch_file.as_os_str().as_bytes())?;
    let target_cstr = CString::new(target_elf.as_os_str().as_bytes())?;
    let request = ffi::PatchLoadRequest {
        patch_file: patch_cstr.as_ptr(),
        target_elf: target_cstr.as_ptr(),
    };
    unsafe {
        ffi::ioctl_load_patch(ioctl_fd, &request)?;
    }

    Ok(())
}

pub fn remove_patch<P>(fd: RawFd, patch_file: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let patch_file = patch_file.as_ref();
    debug!(
        "Upatch: Ioctl {{ fd: {}, cmd: {}, data: {} }}",
        fd,
        stringify!(UPATCH_REMOVE),
        patch_file.display(),
    );

    let patch_cstr = CString::new(patch_file.as_os_str().as_bytes())?;
    unsafe {
        ffi::ioctl_remove_patch(fd, patch_cstr.as_ptr())?;
    }

    Ok(())
}

pub fn active_patch<P>(fd: RawFd, patch_file: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let patch_file = patch_file.as_ref();
    debug!(
        "Upatch: Ioctl {{ fd: {}, cmd: {}, data: {} }}",
        fd,
        stringify!(UPATCH_ACTIVE),
        patch_file.display(),
    );

    let patch_cstr = CString::new(patch_file.as_os_str().as_bytes())?;
    unsafe {
        ffi::ioctl_active_patch(fd, patch_cstr.as_ptr())?;
    }

    Ok(())
}

pub fn deactive_patch<P>(fd: RawFd, patch_file: P) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
    let patch_file = patch_file.as_ref();
    debug!(
        "Upatch: Ioctl {{ fd: {}, cmd: {}, data: {} }}",
        fd,
        stringify!(UPATCH_DEACTIVE),
        patch_file.display(),
    );

    let patch_cstr = CString::new(patch_file.as_os_str().as_bytes())?;
    unsafe {
        ffi::ioctl_deactive_patch(fd, patch_cstr.as_ptr())?;
    }

    Ok(())
}
