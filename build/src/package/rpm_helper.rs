use crate::statics::*;
use crate::util::fs;

use super::RpmBuildRoot;

pub struct RpmHelper;

impl RpmHelper {
    pub fn query_package_info(pkg_path: &str, format: &str) -> std::io::Result<String> {
        fs::check_file(pkg_path)?;

        let exit_status = RPM.execvp([ "--query", "--queryformat", format, pkg_path ])?;

        let exit_code = exit_status.exit_code();
        if exit_code != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Process '{}' exited unsuccessfully, exit code: {}", RPM, exit_code),
            ));
        }

        Ok(exit_status.stdout().to_owned())
    }

    pub fn extract_package(pkg_path: &str, output_dir: &str) -> std::io::Result<RpmBuildRoot> {
        #[inline(always)]
        fn install_package(pkg_path: &str, root_path: &str) -> std::io::Result<()> {
            let exit_status = RPM.execvp([ "--install", "--allfiles", "--root", root_path, pkg_path ])?;
            let exit_code = exit_status.exit_code();

            if exit_code != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Process '{}' exited unsuccessfully, exit code: {}", RPM, exit_code),
                ));
            }

            Ok(())
        }

        #[inline(always)]
        fn patch_package_source(build_root: &RpmBuildRoot) -> std::io::Result<()> {
            let spec_file_path = build_root.find_spec_file()?;
            let exit_status = RPM_BUILD.execvp([
                "--define", &format!("_topdir {}", build_root),
                "-bp", &spec_file_path
            ])?;

            let exit_code = exit_status.exit_code();
            if exit_code != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Process '{}' exited unsuccessfully, exit code: {}", RPM_BUILD, exit_code),
                ));
            }

            Ok(())
        }

        fs::check_file(pkg_path)?;
        fs::check_dir(output_dir)?;

        install_package(pkg_path, output_dir)?;

        let build_root = RpmBuildRoot::new(
            &fs::stringtify_path(
                fs::find_directory(output_dir, "rpmbuild", false, true)?
            )
        );

        patch_package_source(&build_root)?;

        Ok(build_root)
    }
}
