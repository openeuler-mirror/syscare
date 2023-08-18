use std::path::PathBuf;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{error, info};

use syscare_common::os;
use syscare_common::util::fs;

use super::kexec;

lazy_static! {
    static ref BOOT_DIRECTORY: PathBuf = PathBuf::from("/boot");
}

pub enum RebootOption {
    Normal,
    Forced,
}

struct LoadKernelOption {
    name: String,
    kernel: PathBuf,
    initramfs: PathBuf,
}

pub struct KExecManager;

impl KExecManager {
    fn find_kernel(kernel_version: &str) -> Result<LoadKernelOption> {
        info!("Finding kernel \"{}\"...", kernel_version);
        let kernel_file_name = format!("vmlinuz-{}", kernel_version);
        let kernel_file = fs::find_file(
            BOOT_DIRECTORY.as_path(),
            &kernel_file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )
        .with_context(|| format!("Cannot find kernel \"{}\"", kernel_version))?;

        info!("Finding initramfs...");
        let initramfs_file_name = format!("initramfs-{}.img", kernel_version);
        let initramfs_file = fs::find_file(
            BOOT_DIRECTORY.as_path(),
            &initramfs_file_name,
            fs::FindOptions {
                fuzz: false,
                recursive: false,
            },
        )
        .with_context(|| format!("Cannot find kernel \"{}\" initramfs", kernel_version))?;

        Ok(LoadKernelOption {
            name: kernel_version.to_owned(),
            kernel: kernel_file,
            initramfs: initramfs_file,
        })
    }

    fn find_kernel_by_grub() -> Result<LoadKernelOption> {
        info!("Parsing grub configuration...");
        let entry = os::grub::get_boot_entry().context("Failed to read grub boot entry")?;
        let entry_name = entry
            .get_name()
            .to_str()
            .context("Failed to parse grub entry name")?;

        Ok(LoadKernelOption {
            name: entry_name.to_owned(),
            kernel: entry.get_kernel(),
            initramfs: entry.get_initrd(),
        })
    }

    pub fn load_kernel(kernel_version: Option<String>) -> Result<()> {
        let load_option = match kernel_version {
            Some(version) => Self::find_kernel(&version),
            None => Self::find_kernel_by_grub().or_else(|e| {
                error!("{:?}", e);
                let version: &str = os::kernel::version()
                    .to_str()
                    .context("Failed to parse current kernel version")?;

                Self::find_kernel(version)
            }),
        }?;

        kexec::unload().context("Failed to unload kernel")?;

        let name = load_option.name;
        let kernel = load_option.kernel;
        let initramfs = load_option.initramfs;
        info!("Loading {:?}", name);
        info!("Using kernel: {:?}", kernel);
        info!("Using initrd: {:?}", initramfs);

        kexec::load(&kernel, &initramfs).context("Failed to load kernel")
    }

    pub fn execute(option: RebootOption) -> Result<()> {
        match option {
            RebootOption::Normal => kexec::systemd_exec(),
            RebootOption::Forced => kexec::force_exec(),
        }
        .context("Failed to execute kernel")
    }
}
