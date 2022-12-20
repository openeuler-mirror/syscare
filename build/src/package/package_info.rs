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

#[derive(Clone)]
#[derive(Debug)]
pub struct PackageInfo {
    name:       String,
    kind:       PackageType,
    arch:       String,
    epoch:      String,
    version:    String,
    release:    String,
    license:    String,
    source_rpm: String,
}

impl PackageInfo {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> PackageType {
        self.kind
    }

    pub fn get_arch(&self) -> &str {
        &self.arch
    }

    pub fn get_epoch(&self) -> &str {
        &self.epoch
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_release(&self) -> &str {
        &self.release
    }

    pub fn get_license(&self) -> &str {
        &self.license
    }

    pub fn get_source_rpm(&self) -> &str {
        &self.source_rpm
    }
}

impl PackageInfo {
    pub fn get_simple_name(&self) -> String {
        format!("{}-{}-{}",
            self.get_name(),
            self.get_version(),
            self.get_release()
        )
    }

    pub fn query_from(pkg_path: &str) -> std::io::Result<Self> {
        Ok(RpmHelper::query_package_info(
            pkg_path,
            "%{NAME}|%{ARCH}|%{EPOCH}|%{VERSION}|%{RELEASE}|%{LICENSE}|%{SOURCERPM}"
        )?.parse::<PackageInfo>()?)
    }

    pub fn to_query_str(&self) -> String {
        format!("{}|{}|{}|{}|{}|{}|{}",
            self.get_name(),
            self.get_arch(),
            self.get_epoch(),
            self.get_version(),
            self.get_release(),
            self.get_license(),
            self.get_source_rpm(),
        )
    }
}

impl std::str::FromStr for PackageInfo {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pkg_info = s.split('|').collect::<Vec<&str>>();
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

        Ok(Self { name, kind, arch, epoch, version, release, license, source_rpm })
    }
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:    {}\n", self.get_name()))?;
        f.write_fmt(format_args!("type:    {}\n", self.get_type()))?;
        f.write_fmt(format_args!("arch:    {}\n", self.get_arch()))?;
        f.write_fmt(format_args!("epoch:   {}\n", self.get_epoch()))?;
        f.write_fmt(format_args!("version: {}\n", self.get_version()))?;
        f.write_fmt(format_args!("release: {}\n", self.get_release()))?;
        f.write_fmt(format_args!("license: {}",   self.get_license()))?;

        Ok(())
    }
}
