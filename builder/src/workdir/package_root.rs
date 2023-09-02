use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{package::PackageBuildRoot, util};

const SOURCE_DIR_NAME: &str = "source";
const DEBUGINFO_DIR_NAME: &str = "debuginfo";
const PATCH_DIR_NAME: &str = "patch";

#[derive(Debug, Clone)]
pub struct PackageRoot {
    pub path: PathBuf,
    pub source: PathBuf,
    pub debuginfo: PathBuf,
    pub patch: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let source = path.join(SOURCE_DIR_NAME);
        let debuginfo = path.join(DEBUGINFO_DIR_NAME);
        let patch = PackageBuildRoot::new(path.join(PATCH_DIR_NAME))?;

        util::create_dir_all(&path)?;
        util::create_dir_all(&source)?;
        util::create_dir_all(&debuginfo)?;

        Ok(Self {
            path,
            source,
            debuginfo,
            patch,
        })
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
