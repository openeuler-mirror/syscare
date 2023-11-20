use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{package::PackageBuildRoot, util};

const SOURCE_DIR_NAME: &str = "source";
const DEBUGINFO_DIR_NAME: &str = "debuginfo";
const BUILD_ROOT_DIR_NAME: &str = "patch";

#[derive(Debug, Clone)]
pub struct PackageRoot {
    pub path: PathBuf,
    pub source: PathBuf,
    pub debuginfo: PathBuf,
    pub build_root: PackageBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let source = path.join(SOURCE_DIR_NAME);
        let debuginfo = path.join(DEBUGINFO_DIR_NAME);
        let build_root = PackageBuildRoot::new(path.join(BUILD_ROOT_DIR_NAME))?;

        util::create_dir_all(&path)?;
        util::create_dir_all(&source)?;
        util::create_dir_all(&debuginfo)?;

        Ok(Self {
            path,
            source,
            debuginfo,
            build_root,
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