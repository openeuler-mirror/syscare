use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::ops::Deref;

use common::util::fs;

use super::patch_root::PatchRoot;
use super::package_root::PackageRoot;

pub trait WorkDirManager {
    fn create_all(&self) -> std::io::Result<()>;
    fn remove_all(&self) -> std::io::Result<()>;
}

pub struct WorkDir {
    pub path:     PathBuf,
    pub log_file: PathBuf,
    pub patch:    PatchRoot,
    pub package:  PackageRoot,
}

impl WorkDir {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path     = path.as_ref().to_path_buf();
        let log_file = path.join("build.log");
        let patch    = PatchRoot::new(path.join("patch"));
        let package  = PackageRoot::new(path.join("package"));

        Self { path, log_file, patch, package }
    }
}

impl WorkDirManager for WorkDir {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        self.patch.create_all()?;
        self.package.create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.package.remove_all()?;
        self.patch.remove_all()?;
        fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}

impl Deref for WorkDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for WorkDir {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
