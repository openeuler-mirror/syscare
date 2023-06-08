use std::ffi::OsStr;
use std::{ffi::OsString, path::PathBuf};

use log::debug;

use common::os;
use common::util::fs;
use common::util::os_str::OsStringExt;

pub enum RebootOption {
    Normal,
    Forced,
}

struct LoadKernelOption {
    name: OsString,
    kernel: PathBuf,
    initrd: PathBuf,
}

pub struct BootManager;

impl BootManager {
    fn find_kernel_by_name<S: AsRef<OsStr>>(name: S) -> std::io::Result<LoadKernelOption> {
        const BOOT_DIR_NAME: &str = "/boot";
        const KERNEL_PREFIX: &str = "vmlinuz-";
        const INITRAMFS_PREFIX: &str = "initramfs-";
        const INITRAMFS_EXTENSION: &str = ".img";

        debug!("Finding kernel {:?}", name.as_ref());
        let boot_dir = PathBuf::from(BOOT_DIR_NAME);
        let kernel = fs::find_file(
            &boot_dir,
            OsString::from(KERNEL_PREFIX).concat(&name),
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;
        let initrd = fs::find_file(
            &boot_dir,
            OsString::from(INITRAMFS_PREFIX)
                .concat(&name)
                .concat(INITRAMFS_EXTENSION),
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )?;

        Ok(LoadKernelOption {
            name: name.as_ref().to_os_string(),
            kernel,
            initrd,
        })
    }

    fn find_kernel_by_grub_config() -> std::io::Result<LoadKernelOption> {
        debug!("Parsing grub configuration");
        let entry = os::grub::get_boot_entry()?;

        Ok(LoadKernelOption {
            name: entry.get_name().to_os_string(),
            kernel: entry.get_kernel(),
            initrd: entry.get_initrd(),
        })
    }

    pub fn load_kernel<S: AsRef<OsStr>>(kernel_version: Option<S>) -> std::io::Result<()> {
        let option = match kernel_version {
            Some(version) => Self::find_kernel_by_name(version),
            None => Self::find_kernel_by_grub_config().or_else(|e| {
                debug!("{}", e);
                Self::find_kernel_by_name(os::kernel::version())
            }),
        }?;
        debug!("Loading {:?}", option.name);
        debug!("Using kernel: {:?}", option.kernel);
        debug!("Using initrd: {:?}", option.initrd);

        os::kernel::load(option.kernel, option.initrd)
    }

    pub fn reboot(option: RebootOption) -> std::io::Result<()> {
        match option {
            RebootOption::Normal => os::kernel::systemd_exec(),
            RebootOption::Forced => os::kernel::direct_exec(),
        }
    }
}
