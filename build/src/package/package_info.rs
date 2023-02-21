use std::path::Path;

use log::log;
use serde::{Serialize, Deserialize};

use crate::constants::*;

use super::rpm_helper::RpmHelper;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy)]
#[derive(PartialEq)]
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
    pub fn new<P: AsRef<Path>>(pkg_path: P) -> std::io::Result<Self> {
        let query_result = RpmHelper::query_package_info(pkg_path,
            "%{NAME}|%{ARCH}|%{EPOCH}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}"
        )?.to_string_lossy().to_string();

        let pkg_info = query_result.split('|').collect::<Vec<_>>();
        if pkg_info.len() < 7 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Parse package info failed")
            ));
        }

        let name       = pkg_info[0].to_owned();
        let arch       = pkg_info[1].to_owned();
        let epoch      = pkg_info[2].to_owned();
        let version    = pkg_info[3].to_owned();
        let release    = pkg_info[4].to_owned();
        let license    = pkg_info[5].to_owned();
        let source_rpm = pkg_info[6].to_owned();

        let kind = match source_rpm == PKG_FLAG_NONE {
            true  => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };

        Ok(Self { name, kind, arch, epoch, version, release, license })
    }

    pub fn short_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }

    pub fn full_name(&self) -> String {
        format!("{}-{}-{}.{}", self.name, self.version, self.release, self.arch)
    }
}

impl PackageInfo {
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