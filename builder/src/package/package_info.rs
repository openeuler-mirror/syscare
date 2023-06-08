use std::path::Path;

use log::log;
use serde::{Deserialize, Serialize};

use super::rpm_helper::RpmHelper;
use super::rpm_helper::PKG_FILE_EXT;
use super::rpm_spec_helper::TAG_VALUE_NONE;

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub arch: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub license: String,
    pub source_pkg: String,
}

impl PackageInfo {
    pub fn new<P: AsRef<Path>>(pkg_path: P) -> std::io::Result<Self> {
        let query_result = RpmHelper::query_package_info(
            pkg_path.as_ref(),
            "%{NAME}|%{ARCH}|%{EPOCH}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}",
        )?
        .to_string_lossy()
        .to_string();

        let pkg_info = query_result.split('|').collect::<Vec<_>>();
        if pkg_info.len() < 7 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Parse package info from \"{}\" failed",
                    pkg_path.as_ref().display()
                ),
            ));
        }

        let name = pkg_info[0].to_owned();
        let arch = pkg_info[1].to_owned();
        let epoch = pkg_info[2].to_owned();
        let version = pkg_info[3].to_owned();
        let release = pkg_info[4].to_owned();
        let license = pkg_info[5].to_owned();
        let source_pkg = pkg_info[6].to_owned();

        Ok(Self {
            name,
            arch,
            epoch,
            version,
            release,
            license,
            source_pkg,
        })
    }

    pub fn pkg_name(&self) -> String {
        match self.pkg_type() {
            PackageType::SourcePackage => {
                format!(
                    "{}-{}-{}.src.{}",
                    self.name, self.version, self.release, PKG_FILE_EXT
                )
            }
            PackageType::BinaryPackage => {
                format!(
                    "{}-{}-{}.{}.{}",
                    self.name, self.version, self.release, self.arch, PKG_FILE_EXT
                )
            }
        }
    }

    pub fn pkg_type(&self) -> PackageType {
        match self.source_pkg == TAG_VALUE_NONE {
            true => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        }
    }

    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.name, self.version, self.release, self.arch
        )
    }

    pub fn is_source_pkg_of(&self, pkg_info: &PackageInfo) -> bool {
        self.pkg_name() == pkg_info.source_pkg
    }
}

impl PackageInfo {
    pub fn print_log(&self, level: log::Level) {
        log!(level, "name:    {}", self.name);
        log!(level, "type:    {}", self.pkg_type());
        log!(level, "arch:    {}", self.arch);
        log!(level, "epoch:   {}", self.epoch);
        log!(level, "version: {}", self.version);
        log!(level, "release: {}", self.release);
        log!(level, "license: {}", self.license);
    }
}

impl AsRef<PackageInfo> for PackageInfo {
    fn as_ref(&self) -> &PackageInfo {
        self
    }
}
