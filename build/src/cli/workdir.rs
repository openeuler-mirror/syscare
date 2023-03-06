use std::ffi::OsString;
use std::path::Path;

use std::ops::Deref;

use crate::util::os_str::OsStringExt;
use crate::util::sys;

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
                OsString::from(sys::process_name())
                    .concat(".")
                    .concat(sys::process_id().to_string())
            )
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
