use std::path::Path;

use common::util::ext_cmd::ExternCommandArgs;

use super::rpm_helper::RPM;

pub struct RpmExtractor;

impl RpmExtractor {
    pub fn extract_package<P: AsRef<Path>, Q: AsRef<Path>>(pkg_path: P, output_dir: Q) -> std::io::Result<()> {
        RPM.execvp(
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
        )?.check_exit_code()
    }
}
