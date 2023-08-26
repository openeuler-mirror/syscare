use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use super::{fs_util, rpmbuild_root::RpmBuildRoot};

const SOURCE_DIR_NAME: &str = "source";
const DEBUGINFO_DIR_NAME: &str = "debuginfo";
const PATCH_DIR_NAME: &str = "patch";

#[derive(Debug, Clone)]
pub struct PackageRoot {
    pub path: PathBuf,
    pub source: PathBuf,
    pub debuginfo: PathBuf,
    pub patch: RpmBuildRoot,
}

impl PackageRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let source = path.join(SOURCE_DIR_NAME);
        let debuginfo = path.join(DEBUGINFO_DIR_NAME);
        let patch = RpmBuildRoot::new(path.join(PATCH_DIR_NAME))?;

        fs_util::create_dir_all(&path)?;
        fs_util::create_dir_all(&source)?;
        fs_util::create_dir_all(&debuginfo)?;

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
