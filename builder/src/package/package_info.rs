use std::path::Path;

use log::log;
use serde::{Deserialize, Serialize};

use crate::cli::CliArguments;

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
        let kind = match pkg_info[6] == TAG_VALUE_NONE {
            true => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };
        let arch = pkg_info[1].to_owned();
        let epoch = pkg_info[2].to_owned();
        let version = pkg_info[3].to_owned();
        let release = pkg_info[4].to_owned();
        let license = pkg_info[5].to_owned();
        let source_pkg = match kind {
            PackageType::SourcePackage => {
                format!("{}-{}-{}.src.{}", name, version, release, PKG_FILE_EXT)
            }
            PackageType::BinaryPackage => pkg_info[6].to_owned(),
        };

        Ok(Self {
            name,
            kind,
            arch,
            epoch,
            version,
            release,
            license,
            source_pkg,
        })
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

    pub fn is_source_of(&self, pkg_info: &PackageInfo) -> bool {
        if (self.kind == PackageType::SourcePackage)
            && (pkg_info.kind == PackageType::BinaryPackage)
        {
            return self.source_pkg == pkg_info.source_pkg;
        }
        false
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

impl From<&CliArguments> for PackageInfo {
    fn from(args: &CliArguments) -> Self {
        let name = args.target_name.clone().expect("target name is empty");
        let kind = PackageType::SourcePackage;
        let arch = args.target_arch.clone().expect("target arch is empty");
        let epoch = args.target_epoch.clone().expect("target epoch is empty");
        let version = args
            .target_version
            .clone()
            .expect("target version is empty");
        let release = args
            .target_release
            .clone()
            .expect("target release is empty");
        let license = args
            .target_license
            .clone()
            .expect("target license is empty");
        let source_pkg = format!("{}-{}-{}.src.{}", name, version, release, PKG_FILE_EXT);

        Self {
            name,
            kind,
            arch,
            epoch,
            version,
            release,
            license,
            source_pkg,
        }
    }
}
