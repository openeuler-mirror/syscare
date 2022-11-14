use crate::business::cmd::ExternCommand;
use crate::util::fs;

use super::RpmBuildRoot;

pub struct RpmHelper;

const RPM:       ExternCommand = ExternCommand::new("/usr/bin/rpm");
const RPM_BUILD: ExternCommand = ExternCommand::new("/usr/bin/rpmbuild");

impl RpmHelper {
    pub fn query_package_info(pkg_path: &str, format: &str) -> std::io::Result<String> {
        fs::check_file(pkg_path)?;

        let rpm_info = RPM.execvp([ "--query", "--queryformat", format, pkg_path ])?.stdout().to_owned();

        Ok(rpm_info)
    }

    pub fn extract_package(pkg_path: &str, output_dir: &str) -> std::io::Result<RpmBuildRoot> {
        #[inline(always)]
        fn install_package(pkg_path: &str, root_path: &str) -> std::io::Result<()> {
            RPM.execvp([ "--install", "--allfiles", "--root", root_path, pkg_path ])?;
            Ok(())
        }

        #[inline(always)]
        fn patch_package_source(build_root: &RpmBuildRoot) -> std::io::Result<()> {
            let spec_file_path = build_root.find_spec_file()?;
            RPM_BUILD.execvp([
                "--define", &format!("_topdir {}", build_root),
                "-bp", &spec_file_path
            ])?;
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
