use std::path::Path;

use crate::constants::*;
use crate::cmd::ExternCommandArgs;

use super::{PackageInfo, PackageType, RpmBuilder, RpmHelper};

pub struct RpmExtractor;

impl RpmExtractor {
    fn install_package<P: AsRef<Path>, Q: AsRef<Path>>(pkg_path: P, output_dir: Q) -> std::io::Result<()> {
        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("--install")
                .arg("--nodeps")
                .arg("--nofiledigest")
                .arg("--nocontexts")
                .arg("--nocaps")
                .arg("--noscripts")
                .arg("--notriggers")
                .arg("--allfiles")
                .arg("--root")
                .arg(output_dir.as_ref())
                .arg(pkg_path.as_ref())
        )?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit_code={}", RPM, exit_code),
            ));
        }

        Ok(())
    }

    pub fn extract_package<P: AsRef<Path>, Q: AsRef<Path>>(pkg_path: P, output_dir: Q) -> std::io::Result<PackageInfo> {
        Self::install_package(&pkg_path, &output_dir)?;

        let pkg_info = PackageInfo::query_from(&pkg_path)?;
        if pkg_info.get_type() == PackageType::SourcePackage {
            RpmBuilder::new(
                RpmHelper::find_build_root(
                    &output_dir
                )?
            ).build_prepare()?;
        }

        Ok(pkg_info)
    }
}
