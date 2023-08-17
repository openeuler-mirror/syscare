use std::ffi::OsString;
use std::path::Path;

use std::ops::Deref;

use syscare_common::os;
use syscare_common::util::os_str::OsStringExt;

use crate::workdir::{WorkDir, WorkDirManager};

pub struct CliWorkDir {
    inner: Option<WorkDir>,
}

impl CliWorkDir {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn create<P: AsRef<Path>>(&mut self, workdir: P) -> std::io::Result<()> {
        let workdir = WorkDir::new(
            workdir.as_ref().join(
                OsString::from(os::process::name())
                    .concat(".")
                    .concat(os::process::id().to_string()),
            ),
        );
        workdir.create_all()?;

        self.inner = Some(workdir);
        Ok(())
    }

    pub fn remove(&mut self) -> std::io::Result<()> {
        if let Some(inner) = self.inner.take() {
            inner.remove_all()?;
        }

        Ok(())
    }
}

impl Deref for CliWorkDir {
    type Target = WorkDir;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Working directory is not exist")
    }
}

impl Default for CliWorkDir {
    fn default() -> Self {
        Self::new()
    }
}
