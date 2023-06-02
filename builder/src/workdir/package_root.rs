use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::ops::Deref;

use common::util::fs;

use super::workdir::WorkDirManager;
use super::package_build_root::PackageBuildRoot;

pub struct PackageRoot {
    pub path:   PathBuf,
    pub source: PathBuf,
    pub debug:  PathBuf,
    pub patch:  PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path   = path.as_ref().to_path_buf();
        let source = path.join("source");
        let debug  = path.join("debuginfo");
        let patch  = PackageBuildRoot::new(path.join("patch"));

        Self { path, source, debug, patch }
    }
}

impl WorkDirManager for PackageRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        fs::create_dir(&self.source)?;
        fs::create_dir(&self.debug)?;
        self.patch.create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.patch.remove_all()?;
        fs::remove_dir_all(&self.path)?;

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
