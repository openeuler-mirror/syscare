use crate::constants::*;
use crate::util::fs;

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
}
