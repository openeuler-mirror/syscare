use crate::statics::*;

use super::rpm_helper::RpmHelper;

#[derive(Clone, Copy)]
#[derive(PartialEq)]
#[derive(Debug)]
pub enum PackageType {
    SourcePackage,
    BinaryPackage,
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

    pub fn set_name(&mut self, value: String) {
        self.name = value;
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn set_version(&mut self, value: String) {
        self.version = value;
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

    pub fn set_license(&mut self, value: String) {
        self.license = value;
    }

    pub fn get_type(&self) -> PackageType {
        self.kind
    }

    pub fn set_type(&mut self, value: PackageType) {
        self.kind = value;
    }
}

impl PackageInfo {
    pub fn read_from_package(pkg_path: &str) -> std::io::Result<Self> {
        RpmHelper::query_package_info(
            pkg_path,
            "%{NAME}-%{VERSION}-%{RELEASE}-%{LICENSE}-%{SOURCERPM}"
        )?.parse()
    }

    pub fn is_kernel_package(&self) -> bool {
        self.get_name() == KERNEL_PKG_NAME
    }
}

impl std::str::FromStr for PackageInfo {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pkg_info = s.split(PKG_NAME_SPLITER).collect::<Vec<&str>>();
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

        let kind = match source_rpm == PKG_FLAG_SOURCE_PKG {
            true  => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };

        Ok(Self { name, version, release, license, kind })
    }
}
