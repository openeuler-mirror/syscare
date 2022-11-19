use crate::constants::*;
use crate::util::fs;

pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn generate_defconfig(source_dir: &str) -> std::io::Result<()> {
        let exit_status = MAKE.execvp([
            "-C", source_dir,
            KERNEL_DEFCONFIG_NAME
        ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", MAKE, exit_code),
            ));
        }

        Ok(())
    }

    pub fn find_kernel_config(directory: &str) -> std::io::Result<String> {
        let config_file_path = fs::find_file(
            directory,
            KERNEL_CONFIG_NAME,
            false,
            true
        )?;

        Ok(fs::stringtify(config_file_path))
    }

    pub fn find_vmlinux_file(directory: &str) -> std::io::Result<String> {
        let vmlinux_file_path = fs::find_file(
            directory,
            KERNEL_ELF_NAME,
            false,
            true
        )?;

        Ok(fs::stringtify(vmlinux_file_path))
    }
}
