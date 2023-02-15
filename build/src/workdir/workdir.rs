use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::ops::Deref;

use crate::constants::*;
use crate::util::fs;

use super::patch_root::PatchRoot;
use super::package_root::PackageRoot;

pub trait ManageWorkDir {
    fn create_all(&self) -> std::io::Result<()>;
    fn remove_all(&self) -> std::io::Result<()>;
}

pub struct WorkDir {
    path:         PathBuf,
    log_file:     PathBuf,
    patch_root:   PatchRoot,
    package_root: PackageRoot,
}

impl WorkDir {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path         = path.as_ref().to_path_buf();
        let log_file     = path.join(CLI_LOG_FILE_NAME);
        let patch_root   = PatchRoot::new(path.join("patch"));
        let package_root = PackageRoot::new(path.join("package"));
        Self { path, log_file, patch_root, package_root }
    }

    pub fn patch_root(&self) -> &PatchRoot {
        &self.patch_root
    }

    pub fn package_root(&self) -> &PackageRoot {
        &self.package_root
    }

    pub fn log_file(&self) -> &Path {
        &self.log_file
    }
}

impl ManageWorkDir for WorkDir {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        self.patch_root.create_all()?;
        self.package_root.create_all()?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        self.package_root.remove_all()?;
        self.patch_root.remove_all()?;
        std::fs::remove_dir_all(&self.path)?;

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
