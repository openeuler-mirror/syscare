use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::ops::Deref;

use crate::util::fs;

use super::workdir::ManageWorkDir;
use super::package_build_root::PackageBuildRoot;

pub struct PackageRoot {
    path:       PathBuf,
    source_pkg: PathBuf,
    debug_pkg:  PathBuf,
    binary_pkg: PathBuf,
    build_root: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path       = path.as_ref().to_path_buf();
        let source_pkg = path.join("source");
        let debug_pkg  = path.join("debuginfo");
        let binary_pkg = path.join("binary");
        let build_root = PackageBuildRoot::new(path.join("rpmbuild"));

        Self { path, source_pkg, debug_pkg, binary_pkg, build_root }
    }

    pub fn source_pkg_dir(&self) -> &Path {
        &self.source_pkg
    }

    pub fn debug_pkg_dir(&self) -> &Path {
        &self.debug_pkg
    }

    pub fn binary_pkg_dir(&self) -> &Path {
        &self.binary_pkg
    }

    pub fn build_root(&self) -> &PackageBuildRoot {
        &self.build_root
    }
}

impl ManageWorkDir for PackageRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        fs::create_dir(&self.source_pkg)?;
        fs::create_dir(&self.debug_pkg)?;
        fs::create_dir(&self.binary_pkg)?;
        self.build_root().create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.build_root().remove_all()?;
        std::fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}

impl Deref for PackageRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for PackageRoot {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
