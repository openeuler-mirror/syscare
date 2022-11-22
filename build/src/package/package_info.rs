use crate::constants::*;

use super::rpm_helper::RpmHelper;

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

#[derive(Debug)]
pub struct PackageInfo {
    name:    String,
    version: String,
    release: String,
    license: String,
    kind:    PackageType,
}

impl PackageInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn set_release(&mut self, value: String) {
        self.release = value;
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_type(&self) -> PackageType {
        self.kind
    }
}

impl PackageInfo {
    pub fn parse_from(pkg_path: &str) -> std::io::Result<Self> {
        Ok(RpmHelper::query_package_info(
            pkg_path,
            "%{NAME}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}"
        )?.parse::<PackageInfo>()?)
    }
}

impl std::str::FromStr for PackageInfo {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pkg_info = s.split('|').collect::<Vec<&str>>();
        if pkg_info.len() < 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Parse package info failed")
            ));
        }

        let name       = pkg_info[0].to_owned();
        let version    = pkg_info[1].to_owned();
        let release    = pkg_info[2].to_owned();
        let license    = pkg_info[3].to_owned();
        let source_rpm = pkg_info[4].to_owned();

        let kind = match source_rpm == PKG_FLAG_NO_SOURCE_PKG {
            true  => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };

        Ok(Self { name, version, release, license, kind })
    }
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:    {}\n", self.get_name()))?;
        f.write_fmt(format_args!("type:    {}\n", self.get_type()))?;
        f.write_fmt(format_args!("version: {}\n", self.get_version()))?;
        f.write_fmt(format_args!("release: {}\n", self.get_release()))?;
        f.write_fmt(format_args!("license: {}",   self.get_license()))?;

        Ok(())
    }
}
