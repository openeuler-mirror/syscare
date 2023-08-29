use std::{
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Result;

use super::fs_util;

const BUILD_DIR_NAME: &str = "build";
const OUTPUT_DIR_NAME: &str = "output";

#[derive(Debug, Clone)]
pub struct PatchRoot {
    pub path: PathBuf,
    pub build: PathBuf,
    pub output: PathBuf,
}

impl PatchRoot {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let path = base_dir.as_ref().to_path_buf();
        let build = path.join(BUILD_DIR_NAME);
        let output = path.join(OUTPUT_DIR_NAME);

        fs_util::create_dir_all(&path)?;
        fs_util::create_dir_all(&build)?;
        fs_util::create_dir_all(&output)?;

        Ok(Self {
            path,
            build,
            output,
        })
    }
}

impl Deref for PatchRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<OsStr> for PatchRoot {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
