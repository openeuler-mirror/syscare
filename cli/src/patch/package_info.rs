use log::log;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum PackageType {
    SourcePackage,
    BinaryPackage,
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub kind: PackageType,
    pub arch: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub license: String,
    pub source_pkg: String,
}

impl PackageInfo {
    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.name, self.version, self.release, self.arch
        )
    }

    pub fn print_log(&self, level: log::Level) {
        log!(level, "name:    {}", self.name);
        log!(level, "type:    {}", self.kind);
        log!(level, "arch:    {}", self.arch);
        log!(level, "epoch:   {}", self.epoch);
        log!(level, "version: {}", self.version);
        log!(level, "release: {}", self.release);
        log!(level, "license: {}", self.license);
    }
}
