use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use super::fs_util;

const BUILD_DIR_NAME: &str = "BUILD";
const BUILDROOT_DIR_NAME: &str = "BUILDROOT";
const RPMS_DIR_NAME: &str = "RPMS";
const SOURCES_DIR_NAME: &str = "SOURCES";
const SPECS_DIR_NAME: &str = "SPECS";
const SRPMS_DIR_NAME: &str = "SRPMS";

#[derive(Debug, Clone)]
pub struct RpmBuildRoot {
    pub path: PathBuf,
    pub build: PathBuf,
    pub buildroot: PathBuf,
    pub rpms: PathBuf,
    pub sources: PathBuf,
    pub specs: PathBuf,
    pub srpms: PathBuf,
}

impl RpmBuildRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let build = path.join(BUILD_DIR_NAME);
        let buildroot = path.join(BUILDROOT_DIR_NAME);
        let rpms = path.join(RPMS_DIR_NAME);
        let sources = path.join(SOURCES_DIR_NAME);
        let specs = path.join(SPECS_DIR_NAME);
        let srpms = path.join(SRPMS_DIR_NAME);

        fs_util::create_dir_all(&path)?;
        fs_util::create_dir_all(&build)?;
        fs_util::create_dir_all(&buildroot)?;
        fs_util::create_dir_all(&rpms)?;
        fs_util::create_dir_all(&sources)?;
        fs_util::create_dir_all(&specs)?;
        fs_util::create_dir_all(&srpms)?;

        Ok(Self {
            path,
            build,
            buildroot,
            rpms,
            sources,
            specs,
            srpms,
        })
    }
}

impl Deref for RpmBuildRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for RpmBuildRoot {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
