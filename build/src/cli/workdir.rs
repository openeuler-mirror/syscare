use std::path::Path;

use crate::util::sys;
use crate::util::fs;

struct WorkDir {
    work_dir:           String,
    patch_build_root:   String,
    patch_output_dir:   String,
    package_build_root: String,
}

impl WorkDir {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> std::io::Result<Self> {
        let base_dir_path      = fs::stringtify_path(base_dir.as_ref().canonicalize()?);
        let work_dir           = format!("{}/{}.{}", base_dir_path, sys::get_process_name(), sys::get_process_id());
        let patch_build_root   = format!("{}/patch_root",   work_dir);
        let patch_output_dir   = format!("{}/patch_output", patch_build_root);
        let package_build_root = format!("{}/pkg_root",     work_dir);

        fs::create_dir_all(&work_dir)?;
        fs::create_dir(&patch_build_root)?;
        fs::create_dir(&patch_output_dir)?;
        fs::create_dir(&package_build_root)?;

        Ok(Self {
            work_dir,
            patch_build_root,
            patch_output_dir,
            package_build_root,
        })
    }

    pub fn clear(&self) -> std::io::Result<()> {
        std::fs::remove_dir_all(&self.work_dir)
    }
}

pub struct CliWorkDir {
    inner: Option<WorkDir>
}

impl CliWorkDir {
    pub fn new() -> Self {
        Self { inner: None }
    }

    fn get_inner(&self) -> &WorkDir {
        self.inner.as_ref().expect("Working directory is not inited")
    }

    pub fn get_work_dir(&self) -> &str {
        &self.get_inner().work_dir
    }

    pub fn get_patch_build_root(&self) -> &str {
        &self.get_inner().patch_build_root
    }

    pub fn get_patch_output_dir(&self) -> &str {
        &self.get_inner().patch_output_dir
    }

    pub fn get_package_build_root(&self) -> &str {
        &self.get_inner().package_build_root
    }
}

impl CliWorkDir {
    pub fn create(&mut self, base_dir: &str) -> std::io::Result<()> {
        self.inner = Some(WorkDir::new(base_dir)?);

        Ok(())
    }

    pub fn clean_all(&mut self) -> std::io::Result<()> {
        self.get_inner().clear()?;

        Ok(())
    }
}
