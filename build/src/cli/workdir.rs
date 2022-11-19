use std::ops::Deref;

use crate::util::sys;

use crate::workdir::{WorkDir, ManageWorkDir};

pub struct CliWorkDir {
    inner: Option<WorkDir>,
}

impl CliWorkDir {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn create(&mut self, work_dir: &str) -> std::io::Result<()> {
        let process_id   = sys::get_process_id();
        let process_name = sys::get_process_name();

        let base_dir = format!("{}/{}.{}", work_dir, process_name, process_id);
        let work_dir = WorkDir::new(base_dir);
        work_dir.create_all()?;

        self.inner = Some(work_dir);
        Ok(())
    }

    pub fn remove(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.remove_all().ok();
        }
    }
}

impl Deref for CliWorkDir {
    type Target = WorkDir;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Working directory is not exist")
    }
}
