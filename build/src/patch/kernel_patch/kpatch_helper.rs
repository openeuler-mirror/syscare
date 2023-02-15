use std::path::{PathBuf, Path};

use log::debug;

use crate::constants::*;
use crate::util::fs;

use crate::cmd::ExternCommandArgs;

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_defconfig<P: AsRef<Path>>(source_dir: P) -> std::io::Result<()> {
        let exit_status = MAKE.execvp(ExternCommandArgs::new()
            .arg("-C")
            .arg(source_dir.as_ref())
            .arg(KERNEL_DEFCONFIG_NAME)
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", MAKE, exit_code),
            ));
        }

        Ok(())
    }

    pub fn find_kernel_config<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding kernel config from '{}'", directory.as_ref().display());

        fs::find_file(
            directory,
            KERNEL_CONFIG_NAME,
            false,
            true
        )
    }

    pub fn find_vmlinux_file<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding vmlinux from '{}'", directory.as_ref().display());

        fs::find_file(
            directory,
            KERNEL_VMLINUX_FILE,
            false,
            true
        )
    }
}
