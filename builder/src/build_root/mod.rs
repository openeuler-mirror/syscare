use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::util;

mod package_root;
mod patch_root;

pub use package_root::*;
pub use patch_root::*;

const PACKAGE_ROOT_NAME: &str = "package";
const PATCH_ROOT_NAME: &str = "patch";
const BUILD_LOG_NAME: &str = "build.log";

#[derive(Debug, Clone)]
pub struct BuildRoot {
    pub path: PathBuf,
    pub package: PackageRoot,
    pub patch: PatchRoot,
    pub log_file: PathBuf,
}

impl BuildRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let package = PackageRoot::new(path.join(PACKAGE_ROOT_NAME))?;
        let patch = PatchRoot::new(path.join(PATCH_ROOT_NAME))?;
        let log_file = path.join(BUILD_LOG_NAME);

        Ok(Self {
            path,
            log_file,
            patch,
            package,
        })
    }

    pub fn remove(&self) -> Result<()> {
        util::remove_dir_all(&self.path)
    }
}

impl Deref for BuildRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for BuildRoot {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}