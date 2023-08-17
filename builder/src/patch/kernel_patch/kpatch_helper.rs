use std::path::{Path, PathBuf};

use log::debug;
use syscare_common::util::ext_cmd::{ExternCommand, ExternCommandArgs};
use syscare_common::util::fs;

pub const VMLINUX_FILE_NAME: &str = "vmlinux";
pub const KPATCH_PATCH_PREFIX: &str = "syscare";
pub const KPATCH_PATCH_SUFFIX: &str = "ko";

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_defconfig<P: AsRef<Path>>(source_dir: P) -> std::io::Result<()> {
        const MAKE: ExternCommand = ExternCommand::new("make");
        const DEFCONFIG_FILE_NAME: &str = "openeuler_defconfig";

        debug!("Generating kernel default config");

        MAKE.execvp(
            ExternCommandArgs::new()
                .arg("-C")
                .arg(source_dir.as_ref())
                .arg(DEFCONFIG_FILE_NAME),
        )?
        .check_exit_code()
    }

    pub fn find_kernel_config<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        const KERNEL_CONFIG_FILE_NAME: &str = ".config";

        debug!(
            "Finding kernel config from \"{}\"",
            directory.as_ref().display()
        );
        fs::find_file(
            directory,
            KERNEL_CONFIG_FILE_NAME,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )
    }

    pub fn find_vmlinux<P: AsRef<Path>>(directory: P) -> std::io::Result<PathBuf> {
        debug!("Finding vmlinux from \"{}\"", directory.as_ref().display());
        fs::find_file(
            directory,
            VMLINUX_FILE_NAME,
            fs::FindOptions {
                fuzz: false,
                recursive: true,
            },
        )
    }
}
