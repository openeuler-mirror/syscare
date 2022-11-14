use crate::business::cmd::ExternCommand;
use crate::util::fs;

const MAKE: ExternCommand = ExternCommand::new("/usr/bin/make");
const KERNEL_SOURCE_PREFIX:  &str = "linux-";
const KERNEL_DEFCONFIG_NAME: &str = "openeuler_defconfig";
const KERNEL_CONFIG_NAME:    &str = ".config";
const KERNEL_FILE_NAME:      &str = "vmlinux";
pub struct KernelPatchHelper;

impl KernelPatchHelper {
    pub fn find_source_directory(directory: &str) -> std::io::Result<String> {
        fs::check_dir(directory)?;

        let source_dir = fs::find_directory(
            directory,
            KERNEL_SOURCE_PREFIX,
            true,
            true
        )?;

        Ok(fs::stringtify_path(source_dir))
    }

    pub fn find_kernel_config(directory: &str) -> std::io::Result<String> {
        fs::check_dir(directory)?;

        let config_file_path = fs::find_file(
            directory,
            KERNEL_CONFIG_NAME,
            false,
            true
        )?;

        Ok(fs::stringtify_path(config_file_path))
    }

    pub fn generate_defconfig(source_dir: &str) -> std::io::Result<String> {
        fs::check_dir(source_dir)?;

        println!("Using '{}' as default config", KERNEL_DEFCONFIG_NAME);

        MAKE.execvp(["-C", source_dir, KERNEL_DEFCONFIG_NAME])?;
        let config_file_path = fs::find_file(
            source_dir,
            KERNEL_CONFIG_NAME,
            false,
            true
        )?;

        Ok(fs::stringtify_path(config_file_path))
    }

    pub fn write_kernel_config(kconfig_path: &str, output_dir: &str) -> std::io::Result<()> {
        fs::check_file(kconfig_path)?;
        fs::check_dir(output_dir)?;

        let dst_path = format!("{}/{}", output_dir, KERNEL_CONFIG_NAME);
        if kconfig_path.eq(&dst_path) {
            return Ok(());
        }
        std::fs::copy(kconfig_path, dst_path)?;

        Ok(())
    }

    pub fn build_kernel(source_dir: &str) -> std::io::Result<String> {
        fs::check_dir(source_dir)?;

        MAKE.execvp(["-C", source_dir, "clean"])?;
        MAKE.execvp(["-C", source_dir, "-j"])?;

        let kernel_file_path = fs::find_file(
            &source_dir,
            KERNEL_FILE_NAME,
            false,
            true
        )?;

        Ok(fs::stringtify_path(kernel_file_path))
    }
}
