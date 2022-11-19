use crate::constants::*;

use super::{PackageInfo, PackageType, RpmBuilder, RpmHelper};

pub struct RpmExtractor;

impl RpmExtractor {
    fn install_package(pkg_path: &str, output_dir: &str) -> std::io::Result<()> {
        let exit_status = RPM.execvp([
            "--install",
            "--nodeps",
            "--nofiledigest",
            "--nocontexts",
            "--nocaps",
            "--noscripts",
            "--notriggers",
            "--allfiles",
            "--root", output_dir,
            pkg_path
        ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", RPM, exit_code),
            ));
        }

        Ok(())
    }

    fn apply_patch(output_dir: &str) -> std::io::Result<()> {
        RpmBuilder::new(
            RpmHelper::find_build_root(
                output_dir
            )?
        ).build_prepare()
    }

    pub fn extract_package(pkg_path: &str, output_dir: &str) -> std::io::Result<PackageInfo> {
        Self::install_package(pkg_path, output_dir)?;

        let pkg_info = PackageInfo::parse_from(pkg_path)?;
        if pkg_info.get_type() == PackageType::SourcePackage {
            Self::apply_patch(output_dir)?;
        }

        Ok(pkg_info)
    }
}
