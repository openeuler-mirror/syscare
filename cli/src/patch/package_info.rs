use log::log;
use serde::{Serialize, Deserialize};

use crate::util::ext_cmd::{ExternCommand, ExternCommandArgs};

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
    pub name:    String,
    pub kind:    PackageType,
    pub arch:    String,
    pub epoch:   String,
    pub version: String,
    pub release: String,
    pub license: String,
}

impl PackageInfo {
    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!("{}-{}-{}.{}", self.name, self.version, self.release, self.arch)
    }

    pub fn check_installed(&self) -> std::io::Result<()> {
        let pkg_name = self.short_name();

        let exit_status = RPM.execvp(
            ExternCommandArgs::new()
                .arg("-q")
                .arg(&pkg_name)
        )?;

        if exit_status.exit_code() != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Package \"{}\" is not installed", pkg_name)
            ));
        }

        Ok(())
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
