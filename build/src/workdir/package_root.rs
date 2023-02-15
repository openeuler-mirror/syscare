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
    patch_pkg: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path       = path.as_ref().to_path_buf();
        let source_pkg = path.join("source");
        let debug_pkg  = path.join("debuginfo");
        let patch_pkg = PackageBuildRoot::new(path.join("patch"));

        Self { path, source_pkg, debug_pkg, patch_pkg }
    }

    pub fn source_pkg_dir(&self) -> &Path {
        &self.source_pkg
    }

    pub fn debug_pkg_dir(&self) -> &Path {
        &self.debug_pkg
    }

    pub fn patch_pkg_dir(&self) -> &PackageBuildRoot {
        &self.patch_pkg
    }
}

impl ManageWorkDir for PackageRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        fs::create_dir(&self.source_pkg)?;
        fs::create_dir(&self.debug_pkg)?;
        self.patch_pkg_dir().create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.patch_pkg_dir().remove_all()?;
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
