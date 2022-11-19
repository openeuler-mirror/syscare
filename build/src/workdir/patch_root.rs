use crate::util::fs;

use super::workdir::ManageWorkDir;

pub struct PatchRoot {
    base:       String,
    build_root: String,
    output:     String,
}

impl PatchRoot {
    pub fn new(base_dir: String) -> Self {
        Self {
            base:       base_dir.to_owned(),
            build_root: format!("{}/build",  base_dir),
            output:     format!("{}/output", base_dir)
        }
    }

    fn base_dir(&self) -> &str {
        &self.base
    }

    pub fn build_root_dir(&self) -> &str {
        &self.build_root
    }

    pub fn output_dir(&self) -> &str {
        &self.output
    }
}

impl ManageWorkDir for PatchRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(self.base_dir())?;
        fs::create_dir(self.build_root_dir())?;
        fs::create_dir(self.output_dir())?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        std::fs::remove_dir_all(self.base_dir())?;

        Ok(())
    }
}

impl std::fmt::Display for PatchRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.base_dir())
    }
}
