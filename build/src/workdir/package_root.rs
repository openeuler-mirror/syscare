use crate::util::fs;

use super::workdir::ManageWorkDir;
use super::package_build_root::PackageBuildRoot;

pub struct PackageRoot {
    base:       String,
    source_pkg: String,
    debug_pkg:  String,
    binary_pkg: String,
    build_root: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new(base_dir: String) -> Self {
        Self {
            base:       base_dir.to_owned(),
            source_pkg: format!("{}/source",    base_dir),
            debug_pkg:  format!("{}/debuginfo", base_dir),
            binary_pkg: format!("{}/binary",    base_dir),
            build_root: PackageBuildRoot::new(format!("{}/rpmbuild", base_dir)),
        }
    }

    fn base_dir(&self) -> &str {
        &self.base
    }

    pub fn source_pkg_dir(&self) -> &str {
        &self.source_pkg
    }

    pub fn debug_pkg_dir(&self) -> &str {
        &self.debug_pkg
    }

    pub fn binary_pkg_dir(&self) -> &str {
        &self.binary_pkg
    }

    pub fn build_root(&self) -> &PackageBuildRoot {
        &self.build_root
    }
}

impl ManageWorkDir for PackageRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(self.base_dir())?;
        fs::create_dir(self.source_pkg_dir())?;
        fs::create_dir(self.debug_pkg_dir())?;
        fs::create_dir(self.binary_pkg_dir())?;
        self.build_root().create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.build_root().remove_all()?;
        std::fs::remove_dir_all(self.base_dir())?;

        Ok(())
    }
}

impl std::fmt::Display for PackageRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.base_dir())
    }
}
