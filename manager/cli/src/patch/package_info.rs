use log::log;
use serde::{Serialize, Deserialize};

use crate::ext_cmd::{ExternCommand, ExternCommandArgs};

const RPM: ExternCommand = ExternCommand::new("rpm");

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
#[derive(Debug)]
pub enum PackageType {
    SourcePackage,
    BinaryPackage,
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct PackageInfo {
    name:    String,
    kind:    PackageType,
    arch:    String,
    epoch:   String,
    version: String,
    release: String,
    license: String,
}

impl PackageInfo {
    pub fn check_installed(&self) -> std::io::Result<()> {
        let pkg_name = self.get_name();

        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("-q")
                .arg(&pkg_name)
        )?;

        if exit_status.exit_code() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("package \"{}\" is not installed", pkg_name)
            ));
        }

        Ok(())
    }

    pub fn get_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }
}

impl PackageInfo {
    pub fn print_log(&self, level: log::Level) {
        log!(level, "name:    {}", self.name);
        log!(level, "arch:    {}", self.arch);
        log!(level, "epoch:   {}", self.epoch);
        log!(level, "version: {}", self.version);
        log!(level, "release: {}", self.release);
        log!(level, "license: {}", self.license);
    }
}
