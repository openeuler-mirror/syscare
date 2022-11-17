use crate::util::fs;

struct WorkDir {
    work_dir:           String,
    patch_build_root:   String,
    patch_output_dir:   String,
    package_build_root: String,
}

impl WorkDir {
    pub fn new(base: Option<String>) -> std::io::Result<Self> {
        let process_id   = std::process::id();
        let process_name = fs::stringtify_path(std::env::current_exe().unwrap().file_name().unwrap());

        let base_dir = base.unwrap_or(fs::stringtify_path(std::env::current_dir()?));
        let work_dir = format!("{}/{}.{}", base_dir, process_name, process_id);
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
    pub fn create(&mut self, base: Option<String>) -> std::io::Result<()> {
        self.inner = Some(WorkDir::new(base)?);

        Ok(())
    }

    pub fn clean_all(&mut self) -> std::io::Result<()> {
        self.get_inner().clear()?;

        Ok(())
    }
}
