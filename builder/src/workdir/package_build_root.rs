use std::path::{Path, PathBuf};

use syscare_common::util::fs;

use super::workdir_impl::WorkDirManager;

#[derive(Clone, Debug)]
pub struct PackageBuildRoot {
    pub path: PathBuf,
    pub build: PathBuf,
    pub build_root: PathBuf,
    pub rpms: PathBuf,
    pub sources: PathBuf,
    pub specs: PathBuf,
    pub srpms: PathBuf,
}

impl PackageBuildRoot {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let build = path.join("BUILD");
        let build_root = path.join("BUILDROOT");
        let rpms = path.join("RPMS");
        let sources = path.join("SOURCES");
        let specs = path.join("SPECS");
        let srpms = path.join("SRPMS");

        Self {
            path,
            build,
            build_root,
            rpms,
            sources,
            specs,
            srpms,
        }
    }
}

impl WorkDirManager for PackageBuildRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(&self.path)?;
        fs::create_dir(&self.build)?;
        fs::create_dir(&self.build_root)?;
        fs::create_dir(&self.rpms)?;
        fs::create_dir(&self.sources)?;
        fs::create_dir(&self.specs)?;
        fs::create_dir(&self.srpms)?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        fs::remove_dir_all(&self.path)?;

        Ok(())
    }
}

impl AsRef<Path> for PackageBuildRoot {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
