use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
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
    pub fn get_simple_name(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.release)
    }
}

impl std::fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("name:    {}\n", self.name))?;
        f.write_fmt(format_args!("arch:    {}\n", self.arch))?;
        f.write_fmt(format_args!("epoch:   {}\n", self.epoch))?;
        f.write_fmt(format_args!("version: {}\n", self.version))?;
        f.write_fmt(format_args!("release: {}",   self.release))?;

        Ok(())
    }
}
