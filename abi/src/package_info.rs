use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
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
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "name:    {}", self.name)?;
        writeln!(f, "type:    {}", self.kind)?;
        writeln!(f, "arch:    {}", self.arch)?;
        writeln!(f, "epoch:   {}", self.epoch)?;
        writeln!(f, "version: {}", self.version)?;
        writeln!(f, "release: {}", self.release)?;
        write!(f, "license: {}", self.license)?;

        Ok(())
    }
}
