use log::{log, Level};
use std::path::Path;

use syscare_abi::{PackageInfo, PackageType};
use syscare_common::util::fs;

use super::{rpm_spec_helper::TAG_VALUE_NONE, RpmHelper};

pub struct PackageHelper;

impl PackageHelper {
    pub fn parse_pkg_info<P: AsRef<Path>>(pkg_path: P) -> std::io::Result<PackageInfo> {
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
            // For source package, it doesn't have %SOURCERPM, we reuse this field to store file name
            PackageType::SourcePackage => fs::file_name(pkg_path).to_string_lossy().to_string(),
            PackageType::BinaryPackage => pkg_info[6].to_owned(),
        };

        Ok(PackageInfo {
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

    pub fn print_pkg_info(pkg_info: &PackageInfo, level: Level) {
        log!(level, "name:    {}", pkg_info.name);
        log!(level, "type:    {}", pkg_info.kind);
        log!(level, "arch:    {}", pkg_info.arch);
        log!(level, "epoch:   {}", pkg_info.epoch);
        log!(level, "version: {}", pkg_info.version);
        log!(level, "release: {}", pkg_info.release);
        log!(level, "license: {}", pkg_info.license);
    }
}
