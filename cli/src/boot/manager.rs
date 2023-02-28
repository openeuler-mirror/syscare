use std::ffi::OsStr;
use std::{path::PathBuf, ffi::OsString};

use log::info;

use crate::os::KExec;
use crate::util::fs;
use crate::util::os_str::OsStrConcat;

pub enum RebootOption {
    Normal,
    Forced,
}

pub struct BootManager;

impl BootManager {
    pub fn load_kernel<S: AsRef<OsStr>>(kernel_version: S) -> std::io::Result<()> {
        const BOOT_DIR_NAME:       &str = "/boot";
        const KERNEL_PREFIX:       &str = "vmlinuz-";
        const INITRAMFS_PREFIX:    &str = "initramfs-";
        const INITRAMFS_EXTENSION: &str = ".img";

        info!("Kernel version:  {}", kernel_version.as_ref().to_string_lossy());
        let boot_dir = PathBuf::from(BOOT_DIR_NAME);
        let kernel = fs::find_file(
            &boot_dir,
            OsString::from(KERNEL_PREFIX).concat(&kernel_version),
            false,
            false
        )?;
        let initramfs = fs::find_file(
            &boot_dir,
            OsString::from(INITRAMFS_PREFIX).concat(&kernel_version).concat(INITRAMFS_EXTENSION),
            false,
            false
        )?;

        info!("Using kernel:    {}", kernel.display());
        info!("Using initramfs: {}", initramfs.display());

        KExec::load_kernel(kernel, initramfs)
    }

    pub fn reboot(option: RebootOption) -> std::io::Result<()> {
        match option {
            RebootOption::Normal => KExec::systemd_exec_kernel(),
            RebootOption::Forced => KExec::direct_exec_kernel(),
        }
    }
}
