use crate::business::package::rpm_helper::RpmHelper;

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
    kind:    PackageType,
    version: String,
    release: String,
    license: String,
}

impl PackageInfo {
    pub fn read_from_package(pkg_path: &str) -> std::io::Result<Self> {
        const QUERY_FORMAT:     &str = "%{NAME}-%{VERSION}-%{RELEASE}-%{LICENSE}-%{SOURCERPM}";
        const RPM_NAME_SPLITER: char = '-';
        const SOURCE_PKG_FLAG:  &str = "(none)";

        let query_result = RpmHelper::query_package_info(pkg_path, QUERY_FORMAT)?;
        let pkg_info     = query_result.split(RPM_NAME_SPLITER).collect::<Vec<&str>>();
        assert_eq!(pkg_info.len(), 5);

        let pkg_type = match pkg_info[4].eq(SOURCE_PKG_FLAG) {
            true  => PackageType::SourcePackage,
            false => PackageType::BinaryPackage,
        };
        let pkg_info = Self {
            name:    pkg_info[0].to_owned(),
            version: pkg_info[1].to_owned(),
            release: pkg_info[2].to_owned(),
            license: pkg_info[3].to_owned(),
            kind:    pkg_type,
        };

        Ok(pkg_info)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> PackageType {
        self.kind
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
}
