use crate::util::fs;

use super::workdir::ManageWorkDir;

#[derive(Clone)]
#[derive(Debug)]
pub struct PackageBuildRoot {
    base:       String,
    build:      String,
    build_root: String,
    rpms:       String,
    sources:    String,
    specs:      String,
    srpms:      String,
}

impl PackageBuildRoot {
    pub fn new(base_dir: String) -> Self {
        let base       = base_dir.to_owned();
        let build      = format!("{}/BUILD",     base_dir);
        let build_root = format!("{}/BUILDROOT", base_dir);
        let rpms       = format!("{}/RPMS",      base_dir);
        let sources    = format!("{}/SOURCES",   base_dir);
        let specs      = format!("{}/SPECS",     base_dir);
        let srpms      = format!("{}/SRPMS",     base_dir);

        Self { base, build, build_root, rpms, sources, specs, srpms }
    }

    fn base_dir(&self) -> &str {
        &self.base
    }

    pub fn build_dir(&self) -> &str {
        &self.build
    }

    pub fn build_root_dir(&self) -> &str {
        &self.build_root
    }

    pub fn sources_dir(&self) -> &str {
        &self.sources
    }

    pub fn specs_dir(&self) -> &str {
        &self.specs
    }

    pub fn rpms_dir(&self) -> &str {
        &self.rpms
    }

    pub fn srpms_dir(&self) -> &str {
        &self.srpms
    }

}

impl ManageWorkDir for PackageBuildRoot {
    fn create_all(&self) -> std::io::Result<()> {
        fs::create_dir(self.base_dir())?;
        fs::create_dir(self.build_dir())?;
        fs::create_dir(self.build_root_dir())?;
        fs::create_dir(self.rpms_dir())?;
        fs::create_dir(self.sources_dir())?;
        fs::create_dir(self.specs_dir())?;
        fs::create_dir(self.srpms_dir())?;

        Ok(())
    }

    fn remove_all(&self) -> std::io::Result<()> {
        std::fs::remove_dir_all(self.base_dir())?;

        Ok(())
    }
}

impl std::fmt::Display for PackageBuildRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.base_dir())
    }
}
