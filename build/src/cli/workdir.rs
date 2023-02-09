use std::ops::Deref;
use std::path::Path;

use crate::util::sys;

use crate::workdir::{WorkDir, ManageWorkDir};

pub struct CliWorkDir {
    inner: Option<WorkDir>,
}

impl CliWorkDir {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn create<P: AsRef<Path>>(&mut self, workdir: P) -> std::io::Result<()> {
        let dir_name = format!("{}.{}", sys::get_process_name(), sys::get_process_id());
        let workdir = WorkDir::new(workdir.as_ref().join(dir_name));

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
