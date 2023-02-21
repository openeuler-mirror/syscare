use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::ops::Deref;

use crate::util::fs;

use super::workdir::WorkDirManager;

pub struct PatchRoot {
    pub path:   PathBuf,
    pub build:  PathBuf,
    pub output: PathBuf,
}

impl PatchRoot {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        let path   = base_dir.as_ref().to_path_buf();
        let build  = path.join("build");
        let output = path.join("output");

        Self { path, build, output }
    }
}

impl WorkDirManager for PatchRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        fs::create_dir(&self.build)?;
        fs::create_dir(&self.output)?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        fs::remove_dir_all(&self.path)?;

        Ok(())
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
